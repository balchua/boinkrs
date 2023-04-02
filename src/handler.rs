use std::collections::BTreeMap;

use crate::Arguments;
use k8s_openapi::{api::apps::v1::Deployment, serde_json};
use log::{debug, error, info, warn};

use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    core::ObjectList,
    //runtime::wait::{await_condition, conditions::is_pod_running},
    Client,
    Error,
};

pub async fn process(args: &Arguments) -> anyhow::Result<()> {
    let ns = args.get_namespace();
    debug!("Scanning on namespace [{}]", ns);
    let client = Client::try_default().await?;
    let dep_client: Api<Deployment> = Api::namespaced(client, ns);

    let deployments = find_deployments(&dep_client);
    for dep in deployments.await? {
        debug!("Deployment[{}] is found", dep.name_any());
        scale(&dep_client, &dep, args).await?;
    }
    Ok(())
}

async fn find_deployments(dep_client: &Api<Deployment>) -> Result<ObjectList<Deployment>, Error> {
    //let lp = ListParams::default().fields(&format!("metadata.name={}", "norse")); // only want results for our pod
    let lp = ListParams::default();
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
