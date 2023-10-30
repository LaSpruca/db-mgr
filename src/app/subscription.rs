use bollard::Docker;
use futures::StreamExt;
use iced::Subscription;
use iced_futures::{core::Hasher, subscription::Recipe};

use crate::docker::{
    create_container as docker_create_container, CreateContainerEvent, DbContainerConfig,
};

pub fn create_container(
    docker: &'static Docker,
    container_config: DbContainerConfig,
) -> Subscription<CreateContainerEvent> {
    Subscription::from_recipe(DockerSpawn {
        container_config,
        docker,
    })
}

struct DockerSpawn {
    docker: &'static Docker,
    container_config: DbContainerConfig,
}

impl Recipe for DockerSpawn {
    type Output = CreateContainerEvent;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        self.container_config.name.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced_futures::subscription::EventStream,
    ) -> iced_futures::BoxStream<Self::Output> {
        docker_create_container(self.docker, self.container_config).boxed()
    }
}
