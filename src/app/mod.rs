mod add_container;
mod cantainer_card;

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
    widget::{button, column, container, image::Handle, row, scrollable, text, vertical_rule},
    Application, Command, Element, Length, Renderer, Theme,
};
use iced_aw::graphics::icons::ICON_FONT_BYTES;
use itertools::Itertools;
use std::collections::HashMap;

use self::{add_container::add_container, cantainer_card::container_card};

#[derive(Clone, Debug)]
pub enum Message {
    GetContainers,
    GetImages,
    FontLoaded(Result<(), font::Error>),
    Error(String),
    ContainersLoaded(Vec<DbContainer>),
    StartContainer(String),
    StopContainer(String),
    ViewContainer(String),
    LoadedImages(HashMap<String, Handle>),
    ShowCreateContainer,
    CreateContainer(DbContainerConfig),
}

pub enum MainViewState {
    CreateContainer(bool),
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
        };

        (
            this,
            Command::batch([
                font::load(ICON_FONT_BYTES).map(Message::FontLoaded),
                Command::perform(future::ready(()), |_| Message::GetContainers),
                Command::perform(future::ready(()), |_| Message::GetImages),
            ]),
        )
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
            Message::LoadedImages(images) => {
                self.thumbnails = images;
                Command::none()
            }
            Message::ContainersLoaded(containers) => {
                self.containers = containers;
                self.main_view = MainViewState::None;
                Command::none()
            }
            Message::GetImages => Command::perform(
                stream::iter(
                    self.images
                        .clone()
                        .into_iter()
                        .unique_by(|item| item.name.clone()),
                )
                .filter_map(|item| async move {
                    Some((
                        item.name.clone(),
                        Handle::from_memory(
                            reqwest::get(item.icon_url).await.ok()?.bytes().await.ok()?,
                        ),
                    ))
                })
                .collect(),
                Message::LoadedImages,
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
            Message::StartContainer(container) => Command::perform(
                start_container(container, self.docker),
                |result| match result {
                    Err(ex) => Message::Error(format!("Could not start docker container: {ex}")),
                    Ok(_) => Message::GetContainers,
                },
            ),
            Message::StopContainer(container) => Command::perform(
                stop_container(container, self.docker),
                |result| match result {
                    Err(ex) => Message::Error(format!("Could not stop docker container: {ex}")),
                    Ok(_) => Message::GetContainers,
                },
            ),
            Message::ViewContainer(container_name) => {
                self.main_view = self
                    .containers
                    .iter()
                    .enumerate()
                    .find_map(|(i, container)| {
                        if container.name == container_name {
                            Some(MainViewState::ViewContainer(i))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(MainViewState::None);

                Command::none()
            }
            Message::ShowCreateContainer => {
                self.main_view = MainViewState::CreateContainer(false);
                Command::none()
            }
            Message::FontLoaded(_) => Command::none(),
            Message::CreateContainer(_container) => error("Not creating container rn lol"),
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
                                .get(&item.image)
                                .map(|item| item.clone())
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
                    .padding([5, 0, 5, 0]),
            )
            .align_items(iced::Alignment::Center)
            .width(Length::Fill),
        )
        .width(Length::FillPortion(2));

        let main_windown = container(match self.main_view {
            MainViewState::CreateContainer(submit_disabled) => add_container(
                self.images.clone(),
                submit_disabled,
                Message::CreateContainer,
            )
            .into(),
            MainViewState::None => row!().into(),
            MainViewState::ViewContainer(_) => {
                Into::<Element<Message, Renderer>>::into(text("todo"))
            }
        })
        .width(Length::FillPortion(3))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

        row!(containers, vertical_rule(2), main_windown).into()
    }
}
