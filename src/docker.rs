use anyhow::anyhow;
use bollard::{
    container::{Config, CreateContainerOptions, ListContainersOptions},
    errors::Error,
    image::CreateImageOptions,
    service::{ContainerStateStatusEnum, HostConfig, Mount, MountTypeEnum},
    volume::CreateVolumeOptions,
    Docker,
};
use futures::{
    channel::mpsc::{channel, Receiver},
    stream, FutureExt, SinkExt, StreamExt,
};
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

#[derive(Clone, Debug, Hash)]
pub enum CreateContainerEvent {
    Pulling,
    Building,
    Done,
    Error(String),
}

pub fn create_container(
    docker: &'static Docker,
    container_config: DbContainerConfig,
) -> Receiver<CreateContainerEvent> {
    let (mut tx, rx) = channel(5);
    let mut tx2 = tx.clone();

    tokio::spawn((|| {
        async move {
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

            while let Some(result) = image_pull_stream.next().await {
                tx.send(CreateContainerEvent::Pulling).await?;
                #[cfg(debug_assertions)]
                {
                    let result: bollard::service::CreateImageInfo = result?;
                    println!("{result:?}")
                }
                #[cfg(not(debug_assertions))]
                {
                    result?;
                }
            }

            let env = container_config
                .variables
                .into_iter()
                .map(|(name, value)| format!("{name}=\"{value}\""))
                .collect::<Vec<_>>();

            tx.send(CreateContainerEvent::Building).await?;

            for (name, _) in container_config.voluems.iter() {
                create_volume(docker, name).await?;
            }

            let container_name = container_config.name.clone();
            let image = format!("{}:{}", container_config.image, container_config.tag);
            let container = docker
                .create_container(
                    Some(CreateContainerOptions {
                        name: container_config.name,
                        ..Default::default()
                    }),
                    Config {
                        labels: Some(HashMap::from([(LABEL, "container")])),
                        env: Some(env.iter().map(|x| x.as_str()).collect()),
                        image: Some(&image),

                        host_config: Some(HostConfig {
                            mounts: Some(
                                container_config
                                    .voluems
                                    .into_iter()
                                    .map(|(name, path)| Mount {
                                        read_only: Some(false),
                                        target: Some(path),
                                        source: Some(name),
                                        typ: Some(MountTypeEnum::VOLUME),
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

            docker
                .start_container::<String>(&container_name, None)
                .await?;
            println!("{container:?}");
            Ok(())
        }
        .then(|result: Result<(), anyhow::Error>| async move {
            match result.err() {
                Some(ex) => tx2.send(CreateContainerEvent::Error(format!("{ex}"))).await,
                None => tx2.send(CreateContainerEvent::Done).await,
            }
        })
    })());

    rx
}

pub async fn get_containers(docker: &Docker) -> anyhow::Result<Vec<DbContainer>> {
    Ok(stream::iter(
        docker
            .list_containers(Some(ListContainersOptions {
                filters: HashMap::from([("label".into(), vec![format!("{LABEL}=container")])]),
                all: true,
                ..Default::default()
            }))
            .await?,
    )
    .filter_map(|summary| async {
        let out = docker.inspect_container(summary.id?.as_ref(), None).await;

        out.ok()
    })
    .filter_map(|result| async {
        Some(DbContainer {
            id: result.id?,
            name: result.name?,
            image: result
                .config
                .as_ref()
                .map(|config| config.image.clone())??,
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

pub async fn start_container(id: String, docker: &Docker) -> anyhow::Result<()> {
    docker.start_container::<String>(&id, None).await?;

    return Ok(());
}

pub async fn stop_container(id: String, docker: &Docker) -> anyhow::Result<()> {
    docker.stop_container(&id, None).await?;

    return Ok(());
}
