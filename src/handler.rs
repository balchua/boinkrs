use std::collections::BTreeMap;

use crate::{Arguments, Command};
use k8s_openapi::api::apps::v1::Deployment;

use kube::{
    api::{Api, ListParams, ResourceExt},
    //runtime::wait::{await_condition, conditions::is_pod_running},
    Client,
};

fn namespace(args: &Arguments) -> &String {
    match &args.cmd {
        Command::Start { namespace } => {
            return namespace;
        }
        Command::Stop { namespace } => {
            return namespace;
        }
    }
}
pub async fn process(args: &Arguments) -> anyhow::Result<()> {
    let ns = namespace(&args);
    println!("Found namespace: {}", ns);
    let client = Client::try_default().await?;
    let deployment: Api<Deployment> = Api::namespaced(client, ns);

    //let lp = ListParams::default().fields(&format!("metadata.name={}", "norse")); // only want results for our pod
    let lp = ListParams::default();
    let deps = deployment.list(&lp);

    for n in deps.await? {
        println!("Found Deployment: {}", n.name_any());

        for (key, value) in n.labels() {
            println!("Found label: {key}, {value}");
        }

        let t_replicas = target_replicas(n.annotations());
        println!("target replicas {t_replicas}")
    }
    Ok(())
}

fn scale_deployment() {}

fn target_replicas(annotations: &BTreeMap<String, String>) -> u32 {
    let mut found = false;
    for (key, value) in annotations {
        println!("annotation {key}");
        if key == "bal.io/target-replicas" {
            println!("annotation found.");
            return value.parse::<u32>().unwrap();
        }
    }
    0
}
