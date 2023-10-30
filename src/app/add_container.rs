use std::collections::HashMap;

use iced::{
    theme::Text,
    widget::{
        button, checkbox, column, component, pick_list, row, scrollable, text, text_input,
        Component,
    },
    Color, Element, Length, Renderer,
};
use iced_aw::{badge, BadgeStyles};

use crate::{data::DatabaseConfig, docker::DbContainerConfig};

#[derive(Clone)]
pub enum Event {
    SelectContainer(DatabaseConfig),
    SelectedTag(String),
    NameChanged(String),
    EnvVarChanged { key: String, value: String },
    Persist(bool),
    SubmitPressed,
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    None,
    Ready,
    Pulling,
    Creating,
}

pub struct AddContainer<Message> {
    images: Vec<DatabaseConfig>,
    on_add: Box<dyn Fn(DbContainerConfig) -> Message>,
    button_state: ButtonState,
}

#[derive(Debug)]
pub struct AddContainerState {
    data: Option<(DbContainerConfig, DatabaseConfig)>,
    persist: bool,
}

impl Default for AddContainerState {
    fn default() -> Self {
        Self {
            data: None,
            persist: true,
        }
    }
}

pub fn add_container<Message, Handler>(
    images: Vec<DatabaseConfig>,
    button_state: ButtonState,
    on_add: Handler,
) -> AddContainer<Message>
where
    Handler: Fn(DbContainerConfig) -> Message + 'static,
{
    AddContainer::new(images, button_state, on_add)
}

impl<Message> AddContainer<Message> {
    pub fn new<Handler>(
        images: Vec<DatabaseConfig>,
        button_state: ButtonState,
        on_add: Handler,
    ) -> Self
    where
        Handler: Fn(DbContainerConfig) -> Message + 'static,
    {
        Self {
            images,
            button_state,
            on_add: Box::new(on_add),
        }
    }
}

impl<Message> Component<Message, Renderer> for AddContainer<Message> {
    type State = AddContainerState;

    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::SelectContainer(image) => {
                state.data = Some((
                    DbContainerConfig {
                        name: "".into(),
                        variables: HashMap::with_capacity(image.variables.len()),
                        image: image.image.clone(),
                        voluems: image.volumes.clone(),
                        tag: image
                            .tags
                            .get(0)
                            .map(|f| f.to_owned())
                            .unwrap_or_else(|| "latest".to_string()),
                    },
                    image,
                ));

                None
            }
            Event::SelectedTag(tag) => {
                if let Some((config, _)) = state.data.as_mut() {
                    config.tag = tag;
                }

                None
            }
            Event::NameChanged(name) => {
                if let Some((config, _)) = state.data.as_mut() {
                    config.name = name.replace(" ", "-");
                }

                None
            }
            Event::EnvVarChanged { key, value } => {
                if let Some((config, _)) = state.data.as_mut() {
                    if value == "" {
                        _ = config.variables.remove(&key);
                    } else {
                        *config.variables.entry(key).or_insert("".into()) = value;
                    }
                }

                None
            }
            Event::SubmitPressed => {
                if let Some((config, _)) = state.data.as_mut() {
                    let mut new_config = config.clone();
                    new_config.voluems = new_config
                        .voluems
                        .into_iter()
                        .map(|(name, value)| (format!("db-mgr__{}__{name}", config.name), value))
                        .collect();

                    new_config.variables = new_config
                        .variables
                        .into_iter()
                        .filter(|(_, value)| value != "")
                        .collect();

                    new_config.name = format!("db-mgr__{}", config.name);

                    let on_add = self.on_add.as_ref();

                    return Some(on_add(new_config));
                }

                println!("Well this is awkward");
                None
            }
            Event::Persist(voluems_state) => {
                state.persist = voluems_state;
                if let Some((config, selected_container)) = state.data.as_mut() {
                    if voluems_state {
                        config.voluems = selected_container.volumes.clone();
                    } else {
                        config.voluems = HashMap::new();
                    }
                }

                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> iced_aw::Element<'_, Self::Event, Renderer> {
        let mut content = column!(pick_list(
            self.images.clone(),
            state
                .data
                .as_ref()
                .map(|(_, selected_database)| selected_database.clone())
                .clone(),
            Event::SelectContainer,
        )
        .placeholder("Choose image")
        .width(200),)
        .align_items(iced::Alignment::Center)
        .spacing(15)
        .padding(15);

        if let Some((config, selecetd_image)) = state.data.as_ref() {
            content = content.push(
                row!(
                    text_input("name", &config.name).on_input(Event::NameChanged),
                    pick_list(
                        selecetd_image.tags.clone(),
                        Some(config.tag.clone()),
                        Event::SelectedTag
                    )
                )
                .spacing(15),
            );

            for (name, variable) in selecetd_image.variables.iter() {
                let value = config
                    .variables
                    .get(variable)
                    .map(|a| a.to_owned())
                    .unwrap_or_default()
                    .into();

                content = content.push(env_var_row(name.clone(), variable.clone(), value));
            }

            content = content.push(checkbox(
                "Presistant container",
                state.persist,
                Event::Persist,
            ));

            if state.persist {
                content = content.push(text("The following mounts will be created").size(20));
                for (name, path) in selecetd_image.volumes.iter() {
                    content = content.push(
                        row!(
                            text(name),
                            text(path)
                                .size(12)
                                .style(Text::Color(Color::from_rgb8(150, 150, 150)))
                        )
                        .align_items(iced::Alignment::Center)
                        .spacing(5),
                    );
                }
            }

            match (self.button_state, config.name.as_str()) {
                (ButtonState::None, _) => {}
                (ButtonState::Ready, "") => {}

                (ButtonState::Ready, _) => {
                    content =
                        content.push(button("Create Container").on_press(Event::SubmitPressed));
                }
                (ButtonState::Pulling, _) => {
                    content = content.push(badge("Pulling").style(BadgeStyles::Success));
                }
                (ButtonState::Creating, _) => {
                    content = content.push(badge("Creating").style(BadgeStyles::Success));
                }
            }
        }

        return scrollable(content).into();
    }
}

impl<'a, Message> From<AddContainer<Message>> for Element<'a, Message, Renderer>
where
    Message: 'a,
{
    fn from(value: AddContainer<Message>) -> Self {
        component(value)
    }
}

fn env_var_row<'a>(name: String, key: String, value: String) -> Element<'a, Event, Renderer> {
    row!(
        column!(
            text(name),
            text(&key)
                .size(12)
                .style(Text::Color(Color::from_rgb8(150, 150, 150)))
        )
        .width(Length::FillPortion(2)),
        text_input(&key, &value)
            .on_input(move |text| {
                let key = key.clone();
                Event::EnvVarChanged { key, value: text }
            })
            .width(Length::FillPortion(3)),
    )
    .align_items(iced::Alignment::Start)
    .into()
}
