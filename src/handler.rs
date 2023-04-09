use std::collections::BTreeMap;

use crate::Arguments;
use k8s_openapi::{api::apps::v1::Deployment, serde_json};
use log::{debug, info};

use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    core::ObjectList,
    Client, Error,
};

pub async fn process(args: &Arguments) -> anyhow::Result<()> {
    let ns = args.get_namespace();
    debug!("Scanning on namespace [{}]", ns);
    let client = Client::try_default().await?;
    let dep_client: Api<Deployment> = Api::namespaced(client, ns);

    let deployments = find_deployments(&dep_client, &args);
    for dep in deployments.await? {
        info!("Deployment[{}] is found", dep.name_any());
        scale(&dep_client, &dep, args).await?;
    }
    Ok(())
}

async fn find_deployments(
    dep_client: &Api<Deployment>,
    args: &Arguments,
) -> Result<ObjectList<Deployment>, Error> {
    let lp = ListParams::default().labels(&args.label); // only want results for our pod
                                                        //let lp = ListParams::default();
    let deps = dep_client.list(&lp).await?;
    return Ok(deps);
}

async fn scale_to_zero(dep_client: &Api<Deployment>, deployment: &Deployment) -> Result<(), Error> {
    info!("Prepare to scale to zero");
    let current = current_replicas(deployment);

    let patch = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
          "annotations": {
            "bal.io/target-replicas": current.to_string(),
          }
        },
        "spec": {
          "replicas": 0,
        }
    });
    let name = deployment.name_any();
    let ssapply = PatchParams::apply("boinkrs").force();
    let out = dep_client
        .patch(&name, &ssapply, &Patch::Apply(&patch))
        .await?;
    debug!("Resulting deployment {:?}", out.spec);
    Ok(())
}
async fn patch_metadata(
    dep_client: &Api<Deployment>,
    deployment: &Deployment,
) -> Result<(), Error> {
    info!("updating annotations and patching the deployment");
    let current = current_replicas(deployment);

    let patch = serde_json::json!({
    "apiVersion": "apps/v1",
    "kind": "Deployment",
    "metadata": {
      "annotations": {
        "bal.io/target-replicas": current.to_string(),
      }
    }
    });
    let name = deployment.name_any();
    let ssapply = PatchParams::apply("boinkrs").force();
    let out = dep_client
        .patch(&name, &ssapply, &Patch::Apply(&patch))
        .await?;
    debug!("Resulting deployment {:?}", out.spec);
    Ok(())
}

async fn scale_up(dep_client: &Api<Deployment>, deployment: &Deployment) -> Result<(), Error> {
    info!("Prepare to scale up");
    let target = target_replicas(deployment.annotations());

    if target == 0 {
        patch_metadata(dep_client, deployment).await?;
        return Ok(());
    }
    let patch = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "spec": {
          "replicas": target,
        }
    });
    let name = deployment.name_any();
    let ssapply = PatchParams::apply("boinkrs").force();
    let out = dep_client
        .patch(&name, &ssapply, &Patch::Apply(&patch))
        .await?;
    debug!("Resulting deployment {:?}", out.spec);
    Ok(())
}

async fn scale(
    dep_client: &Api<Deployment>,
    deployment: &Deployment,
    args: &Arguments,
) -> Result<(), Error> {
    if !args.is_stop_action() {
        scale_up(dep_client, deployment).await?;
        return Ok(());
    }

    scale_to_zero(dep_client, deployment).await?;
    Ok(())
}

fn current_replicas(deployment: &Deployment) -> i32 {
    match &deployment.spec {
        Some(deployment_spec) => {
            return deployment_spec.replicas.unwrap();
        }
        None => return 0,
    }
}

fn target_replicas(annotations: &BTreeMap<String, String>) -> u32 {
    for (key, value) in annotations {
        if key == "bal.io/target-replicas" {
            return value.parse::<u32>().unwrap();
        }
    }
    0
}

#[cfg(test)]
mod test {
    use kube::{core::ObjectList, Api, Client};

    use http::{Request, Response};
    use hyper::{Body, Method};
    use k8s_openapi::{api::apps::v1::Deployment, http, serde_json};
    use tower_test::mock::{self, Handle};

    use crate::{
        handler::{find_deployments, scale_up},
        Arguments, Command,
    };

    async fn patch_deployment(handle: &mut Handle<Request<Body>, Response<Body>>) {
        let (request, send) = handle.next_request().await.expect("Service not called");

        let (parts, _body) = request.into_parts();
        println!("path is {}, method is {}", parts.uri.path(), parts.method);
        let body = match (parts.method, parts.uri.path()) {
            (Method::PATCH, "/apis/apps/v1/namespaces/default/deployments/test") => {
                // Can also use recorded resources by reading from a file.
                // Or create entire mock from some file mapping routes to resources.
                let deployment: ObjectList<Deployment> = serde_json::from_value(serde_json::json!(
                {
                    "kind": "DeploymentList",
                    "apiVersion": "apps/v1",
                    "metadata": {
                      "resourceVersion": "208553"
                    },
                    "items": [],
                }))
                .unwrap();
                serde_json::to_vec(&deployment).unwrap()
            }
            _ => panic!("Unexpected API request {:?}", parts.uri.path()),
        };

        send.send_response(Response::builder().body(Body::from(body)).unwrap());
    }

    async fn mock_get_deployment_with_labels(handle: &mut Handle<Request<Body>, Response<Body>>) {
        let (request, send) = handle.next_request().await.expect("Service not called");

        let body = match (
            request.method().as_str(),
            request.uri().to_string().as_str(),
        ) {
            ("GET", "/apis/apps/v1/namespaces/default/deployments?&labelSelector=app%3Dnginx") => {
                // Can also use recorded resources by reading from a file.
                // Or create entire mock from some file mapping routes to resources.
                let deployment: ObjectList<Deployment> = serde_json::from_value(serde_json::json!(
                {
                    "kind": "DeploymentList",
                    "apiVersion": "apps/v1",
                    "metadata": {
                      "resourceVersion": "208553"
                    },
                    "items": [
                    {
                        "apiVersion": "apps/v1",
                        "kind": "Deployment",
                        "metadata": {
                            "name": "test",
                            "annotations": { "kube-rs": "test" },
                            "labels": {
                                "app": "nginx"
                            }
                        },
                        "spec": {
                            "replicas": 3,
                            "selector": {
                                "matchLabels" : {
                                    "app":"nginx"
                                }
                            },
                            "template" : {
                                "metadata" : {
                                    "labels" : {
                                        "app":"nginx"
                                    }
                                },
                            },
                            "spec":{
                                "containers":[
                                    {
                                        "name":"ngnix",
                                        "image":"nginx:1.7.9",
                                        "ports":[
                                        {
                                            "containerPort": 80
                                        }
                                        ]
                                    }
                                ]
                            },
                        }
                    }
                    ],
                }))
                .unwrap();
                serde_json::to_vec(&deployment).unwrap()
            }
            ("GET", "/apis/apps/v1/namespaces/default/deployments?&labelSelector=app%3Ddummy") => {
                // Can also use recorded resources by reading from a file.
                // Or create entire mock from some file mapping routes to resources.
                let deployment: ObjectList<Deployment> = serde_json::from_value(serde_json::json!(
                {
                    "kind": "DeploymentList",
                    "apiVersion": "apps/v1",
                    "metadata": {
                      "resourceVersion": "208553"
                    },
                    "items": [],
                }))
                .unwrap();
                serde_json::to_vec(&deployment).unwrap()
            }
            _ => panic!("Unexpected API request {:?}", request),
        };

        send.send_response(Response::builder().body(Body::from(body)).unwrap());
    }

    #[tokio::test]
    async fn must_find_deployments_with_correct_label() {
        let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
        let label = "app=nginx";
        let spawned = tokio::spawn(async move {
            mock_get_deployment_with_labels(&mut handle).await;
        });
        let args = &Arguments {
            label: String::from(label),
            cmd: Command::Start {
                namespace: String::from(""),
            },
        };

        let deployments: Api<Deployment> =
            Api::default_namespaced(Client::new(mock_service, "default"));

        let deps = find_deployments(&deployments, args);

        let lists = deps.await.unwrap();
        assert_eq!(1, lists.items.len());
        spawned.await.unwrap();
    }

    #[tokio::test]
    async fn must_find_deployments_with_wrong_label() {
        let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
        let label = "app=dummy";
        let spawned = tokio::spawn(async move {
            mock_get_deployment_with_labels(&mut handle).await;
        });
        let args = &Arguments {
            label: String::from(label),
            cmd: Command::Start {
                namespace: String::from(""),
            },
        };

        let deployments: Api<Deployment> =
            Api::default_namespaced(Client::new(mock_service, "default"));

        let deps = find_deployments(&deployments, args);

        let lists = deps.await.unwrap();
        assert_eq!(0, lists.items.len());
        spawned.await.unwrap();
    }
    #[tokio::test]
    async fn must_scale_deployment() {
        let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();

        let spawned = tokio::spawn(async move {
            patch_deployment(&mut handle).await;
        });

        let deployments: Api<Deployment> =
            Api::default_namespaced(Client::new(mock_service, "default"));

        let d: Deployment = serde_json::from_value(serde_json::json!(
            {
                "apiVersion": "apps/v1",
                "kind": "Deployment",
                "metadata": {
                    "name": "test",
                    "annotations": { "kube-rs": "test" },
                    "labels": {
                        "app": "nginx"
                    }
                },
                "spec": {
                    "replicas": 3,
                    "selector": {
                        "matchLabels" : {
                            "app":"nginx"
                        }
                    },
                    "template" : {
                        "metadata" : {
                            "labels" : {
                                "app":"nginx"
                            }
                        },
                    },
                    "spec":{
                        "containers":[
                            {
                                "name":"ngnix",
                                "image":"nginx:1.7.9",
                                "ports":[
                                {
                                    "containerPort": 80
                                }
                                ]
                            }
                        ]
                    },
                }
        }))
        .unwrap();

        let res = scale_up(&deployments, &d);

        let response = res.await.unwrap();

        match response {
            () => assert!(true),
        }
        spawned.await.unwrap();
    }
}
