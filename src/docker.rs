use anyhow::anyhow;
use bollard::{
    container::{Config, CreateContainerOptions, ListContainersOptions},
    errors::Error,
    image::CreateImageOptions,
    service::{ContainerStateStatusEnum, HostConfig, Mount},
    volume::CreateVolumeOptions,
    Docker,
};
use futures::{stream, StreamExt};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbContainerConfig {
    pub name: String,
    pub variables: HashMap<String, String>,
    pub image: String,
    pub voluems: HashMap<String, String>,
    pub tag: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbContainer {
    pub id: String,
    pub name: String,
    pub state: ContainerStateStatusEnum,
    pub variables: HashMap<String, String>,
    pub image: String,
    pub volumes: HashMap<String, String>,
}

const LABEL: &str = "db-mgr-resource";

async fn create_volume(docker: &Docker, name: &str) -> anyhow::Result<()> {
    match docker.inspect_volume(name).await {
        Err(Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Ok(_) => return Err(anyhow!("Container name conflict {}", name)),
        Err(resp) => return Err(anyhow!(resp)),
    };

    docker
        .create_volume(CreateVolumeOptions {
            name,
            labels: HashMap::from([(LABEL.into(), "volume".into())]),
            ..Default::default()
        })
        .await?;

    Ok(())
}

pub async fn create_container(
    docker: &Docker,
    container_config: DbContainerConfig,
) -> anyhow::Result<()> {
    match docker.inspect_container(&container_config.name, None).await {
        Err(Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Ok(_) => return Err(anyhow!("Container name conflict {}", container_config.name)),
        Err(resp) => return Err(anyhow!(resp)),
    };

    let mut image_pull_stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: container_config.image.as_str(),
            tag: container_config.tag.as_str(),
            ..Default::default()
        }),
        None,
        None,
    );

    if let Some(result) = image_pull_stream.next().await {
        result?;
    }

    let env = container_config
        .variables
        .into_iter()
        .map(|(name, value)| format!("{name}=\"{value}\""))
        .collect::<Vec<_>>();

    for (name, _) in container_config.voluems.iter() {
        create_volume(docker, name).await?;
    }

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_config.name,
                ..Default::default()
            }),
            Config {
                labels: Some(HashMap::from([(LABEL, "container")])),
                env: Some(env.iter().map(|x| x.as_str()).collect()),
                image: Some(container_config.image.as_str()),
                host_config: Some(HostConfig {
                    mounts: Some(
                        container_config
                            .voluems
                            .into_iter()
                            .map(|(name, path)| Mount {
                                read_only: Some(false),
                                target: Some(path),
                                source: Some(name),
                                ..Default::default()
                            })
                            .collect(),
                    ),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}

pub async fn get_containers(docker: &Docker) -> anyhow::Result<Vec<DbContainer>> {
    Ok(stream::iter(
        docker
            .list_containers(Some(ListContainersOptions {
                filters: HashMap::from([("label".into(), vec![format!("{LABEL}=container")])]),
                ..Default::default()
            }))
            .await?,
    )
    .filter_map(|summary| async {
        docker
            .inspect_container(summary.names?.get(0)?, None)
            .await
            .ok()
    })
    .filter_map(|result| async {
        Some(DbContainer {
            id: result.id?,
            name: result.name?,
            image: result.image?,
            state: result.state?.status?,
            volumes: result
                .mounts
                .map(|mounts| {
                    mounts
                        .into_iter()
                        .filter_map(|mount| {
                            Some((mount.name.or_else(|| mount.source)?, mount.destination?))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            variables: result
                .config?
                .env
                .map(|e| {
                    e.into_iter()
                        .map(|entry| match entry.split_once("=") {
                            Some((name, var)) => (name.into(), var.into()),
                            None => (entry, "".into()),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    })
    .collect()
    .await)
}

pub async fn start_container(name: String, docker: &Docker) -> anyhow::Result<()> {
    docker.start_container::<String>(&name, None).await?;

    return Ok(());
}

pub async fn stop_container(name: String, docker: &Docker) -> anyhow::Result<()> {
    docker.stop_container(&name, None).await?;

    return Ok(());
}
