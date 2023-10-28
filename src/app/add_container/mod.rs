use iced::{
    widget::{column, component, pick_list, row, scrollable, text_input, Component},
    Element, Renderer,
};

use crate::{data::DatabaseConfig, docker::DbContainerConfig};

#[derive(Clone)]
pub enum Event {
    SelectContainer(DatabaseConfig),
    SelectedTag(String),
    NameChanged(String),
}

pub struct AddContainer<Message> {
    images: Vec<DatabaseConfig>,
    on_add: Box<dyn Fn(DbContainerConfig) -> Message>,
    input_disabled: bool,
}

#[derive(Debug)]
pub struct AddContainerState {
    selected_image: Option<DatabaseConfig>,
    config: Option<DbContainerConfig>,
    persist: bool,
}

impl Default for AddContainerState {
    fn default() -> Self {
        Self {
            selected_image: None,
            config: None,
            persist: true,
        }
    }
}

pub fn add_container<Message, Handler>(
    images: Vec<DatabaseConfig>,
    input_disabled: bool,
    on_add: Handler,
) -> AddContainer<Message>
where
    Handler: Fn(DbContainerConfig) -> Message + 'static,
{
    AddContainer::new(images, input_disabled, on_add)
}

impl<Message> AddContainer<Message> {
    pub fn new<Handler>(images: Vec<DatabaseConfig>, input_disabled: bool, on_add: Handler) -> Self
    where
        Handler: Fn(DbContainerConfig) -> Message + 'static,
    {
        Self {
            images,
            input_disabled,
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
                state.config = Some(DbContainerConfig {
                    name: image.name.clone(),
                    variables: image
                        .variables
                        .iter()
                        .map(|(_, key)| (key.to_owned(), "".to_owned()))
                        .collect(),
                    image: image.image.clone(),
                    voluems: image.volumes.clone(),
                    tag: image
                        .tags
                        .get(0)
                        .map(|f| f.to_owned())
                        .unwrap_or_else(|| "latest".to_string()),
                });

                state.selected_image = Some(image);

                None
            }
            Event::SelectedTag(tag) => {
                if let Some(config) = state.config.as_mut() {
                    config.tag = tag;
                }

                None
            }
            Event::NameChanged(name) => {
                if let Some(config) = state.config.as_mut() {
                    config.name = name;
                }

                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> iced_aw::Element<'_, Self::Event, Renderer> {
        if let Some(selecetd_container) = state.selected_image.as_ref() {
            let config = state.config.as_ref().unwrap();
            return scrollable(
                column!(
                    pick_list(
                        self.images.clone(),
                        state.selected_image.clone(),
                        Event::SelectContainer,
                    )
                    .placeholder("Choose image")
                    .width(200),
                    row!(
                        text_input("name", &config.name).on_input(Event::NameChanged),
                        pick_list(
                            selecetd_container.tags.clone(),
                            Some(config.tag.clone()),
                            Event::SelectedTag
                        )
                    )
                    .spacing(15)
                )
                .align_items(iced::Alignment::Center)
                .spacing(15)
                .padding(15),
            )
            .into();
        }

        return scrollable(column!(pick_list(
            self.images.clone(),
            state.selected_image.clone(),
            Event::SelectContainer,
        )
        .placeholder("Choose image")))
        .into();
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
