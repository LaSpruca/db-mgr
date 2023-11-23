mod add_container;
mod cantainer_card;
mod container_view;
mod subscription;

use self::{
    add_container::{add_container, ButtonState},
    cantainer_card::container_card,
    container_view::container_view,
    subscription::create_container,
};
use crate::{
    data::{ConfigFile, DatabaseConfig},
    docker::{get_containers, start_container, stop_container, DbContainer, DbContainerConfig},
};
use bollard::Docker;
use futures::{future, stream, StreamExt};
use iced::{
    alignment::{Horizontal, Vertical},
    executor::Default as DefaultExector,
    font,
    widget::{button, column, container, image::Handle, row, scrollable, vertical_rule},
    Application, Command, Length, Subscription, Theme,
};
use iced_aw::graphics::icons::ICON_FONT_BYTES;
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Message {
    GetContainers,
    GetThumbnails,
    FontLoaded(Result<(), font::Error>),
    Error(String),
    ContainersLoaded(Vec<DbContainer>),
    StartContainer(String),
    StopContainer(String),
    ViewContainer(String),
    LoadedThumbnails(HashMap<String, Handle>),
    ShowCreateContainer,
    CreateContainer(DbContainerConfig),
    PullingContainer,
    BuildingContainer,
    BuildError(String),
    CreatedContainer,
    ImageDownload(String, f32),
}

#[derive(Debug)]
pub enum MainViewState {
    CreateContainer(ButtonState),
    ViewContainer(usize),
    None,
}

pub struct DbMgrApp {
    containers: Vec<DbContainer>,
    images: Vec<DatabaseConfig>,
    docker: &'static Docker,
    thumbnails: HashMap<String, Handle>,
    main_view: MainViewState,
    default_thumbnail: Handle,
    build_subscription: Option<DbContainerConfig>,
}

fn error(message: impl Into<String>) -> Command<Message> {
    let str = message.into();
    Command::perform(future::ready(()), move |_| Message::Error(str))
}

fn run(message: Message) -> Command<Message> {
    {
        Command::perform(future::ready(()), move |_| message)
    }
}

impl Application for DbMgrApp {
    type Executor = DefaultExector;

    type Message = Message;

    type Theme = Theme;

    type Flags = (Docker, ConfigFile);

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn new((docker, config_file): Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let this = Self {
            containers: vec![],
            docker: Box::leak(Box::new(docker)),
            thumbnails: HashMap::with_capacity(config_file.databases.len()),
            images: config_file.databases,
            main_view: MainViewState::None,
            default_thumbnail: Handle::from_memory(include_bytes!("../../default_image.png")),
            build_subscription: None,
        };

        (
            this,
            Command::batch([
                font::load(ICON_FONT_BYTES).map(Message::FontLoaded),
                Command::perform(future::ready(()), |_| Message::GetContainers),
                Command::perform(future::ready(()), |_| Message::GetThumbnails),
            ]),
        )
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match self.build_subscription.as_ref() {
            Some(container_config) => create_container(self.docker, container_config.to_owned())
                .map(|event| match event {
                    crate::docker::CreateContainerEvent::Pulling => Message::PullingContainer,
                    crate::docker::CreateContainerEvent::Building => Message::BuildingContainer,
                    crate::docker::CreateContainerEvent::Done => Message::CreatedContainer,
                    crate::docker::CreateContainerEvent::Error(ex) => Message::BuildError(ex),
                    crate::docker::CreateContainerEvent::Download(key, value) => {
                        Message::ImageDownload(key, value)
                    }
                }),
            None => Subscription::none(),
        }
    }

    fn title(&self) -> String {
        "DB Manage".into()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::GetContainers => {
                Command::perform(get_containers(self.docker), |result| match result {
                    Err(ex) => Message::Error(format!("Could not get containers: {ex}")),
                    Ok(containers) => Message::ContainersLoaded(containers),
                })
            }
            Message::LoadedThumbnails(images) => {
                self.thumbnails = images;
                Command::none()
            }
            Message::ContainersLoaded(containers) => {
                self.containers = containers;
                self.main_view = MainViewState::None;
                Command::none()
            }
            Message::GetThumbnails => Command::perform(
                stream::iter(
                    self.images
                        .clone()
                        .into_iter()
                        .unique_by(|item| item.name.clone()),
                )
                .filter_map(|item| async move {
                    Some((
                        item.image.clone(),
                        Handle::from_memory(
                            reqwest::get(item.icon_url).await.ok()?.bytes().await.ok()?,
                        ),
                    ))
                })
                .collect(),
                Message::LoadedThumbnails,
            ),
            Message::Error(ex) => {
                if let Err(dialog_err) = native_dialog::MessageDialog::new()
                    .set_text(&ex)
                    .set_type(native_dialog::MessageType::Error)
                    .show_alert()
                {
                    eprintln!("Application Error: {ex}");
                    eprintln!("Dialog Error: {dialog_err}");
                }
                Command::none()
            }
            Message::StartContainer(id) => {
                Command::perform(start_container(id, self.docker), |result| match result {
                    Err(ex) => Message::Error(format!("Could not start docker container: {ex}")),
                    Ok(_) => Message::GetContainers,
                })
            }
            Message::StopContainer(id) => {
                Command::perform(stop_container(id, self.docker), |result| match result {
                    Err(ex) => Message::Error(format!("Could not stop docker container: {ex}")),
                    Ok(_) => Message::GetContainers,
                })
            }
            Message::ViewContainer(container_name) => {
                self.main_view = self
                    .containers
                    .iter()
                    .enumerate()
                    .find_map(|(i, container)| {
                        if container.id == container_name {
                            Some(MainViewState::ViewContainer(i))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(MainViewState::None);

                println!("{:?}", self.main_view);

                Command::none()
            }
            Message::ShowCreateContainer => {
                self.main_view = MainViewState::CreateContainer(ButtonState::Ready);
                Command::none()
            }
            Message::FontLoaded(_) => Command::none(),
            Message::CreateContainer(container_config) => {
                self.main_view = MainViewState::CreateContainer(ButtonState::Creating);
                // let rx = create_container(self.docker, container_config);

                self.build_subscription = Some(container_config);

                Command::none()
            }
            Message::BuildError(ex) => {
                self.main_view = MainViewState::CreateContainer(ButtonState::Ready);
                self.build_subscription = None;
                error(ex)
            }
            Message::ImageDownload(key, value) => {
                // self.main_view = MainViewState::CreateContainer(ButtonState::Pulling);
                match self.main_view {
                    MainViewState::CreateContainer(ButtonState::Pulling(ref mut status)) => {
                        status.insert(key, value);
                    }
                    MainViewState::ViewContainer(_)
                    | MainViewState::None
                    | MainViewState::CreateContainer(_) => {
                        self.main_view =
                            MainViewState::CreateContainer(ButtonState::Pulling(HashMap::new()))
                    }
                }

                Command::none()
            }
            Message::BuildingContainer => {
                self.main_view = MainViewState::CreateContainer(ButtonState::Creating);
                Command::none()
            }
            Message::CreatedContainer => {
                self.build_subscription = None;
                Command::perform(future::ready(()), |_| Message::GetContainers)
            }
            Message::PullingContainer => {
                self.main_view =
                    MainViewState::CreateContainer(ButtonState::Pulling(HashMap::new()));
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let containers = scrollable(
            column(
                self.containers
                    .iter()
                    .map(|item| {
                        container_card(
                            item,
                            self.thumbnails
                                .get(item.image.split(':').next().unwrap_or(item.image.as_str()))
                                .cloned()
                                .unwrap_or_else(|| self.default_thumbnail.clone()),
                        )
                        .on_start_click(Message::StartContainer)
                        .on_stop_click(Message::StopContainer)
                        .on_view_click(Message::ViewContainer)
                        .into()
                    })
                    .collect(),
            )
            .push(
                container(button("Add container").on_press(Message::ShowCreateContainer))
                    .padding([5, 0]),
            )
            .align_items(iced::Alignment::Center)
            .width(Length::Fill),
        )
        .width(Length::FillPortion(1));

        let main_windown = match self.main_view {
            MainViewState::CreateContainer(ref state) => container(add_container(
                self.images.clone(),
                state.clone(),
                Message::CreateContainer,
            )),
            MainViewState::None => container(row!()),
            MainViewState::ViewContainer(index) => {
                let ctr = &self.containers[index];
                container(container_view(
                    ctr.to_owned(),
                    self.thumbnails
                        .get(&ctr.name)
                        .unwrap_or(&self.default_thumbnail)
                        .to_owned(),
                ))
            }
        }
        .width(Length::FillPortion(2))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

        row!(containers, vertical_rule(2), main_windown).into()
    }
}
