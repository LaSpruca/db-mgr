use crate::docker::DbContainer;
use iced::{
    widget::{column, component, image::Handle, row, scrollable, text, Component, Image},
    Element, Renderer,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub enum Event {}

pub struct ContainerView<Message> {
    container: DbContainer,
    image: Handle,
    t: PhantomData<Message>,
}

#[derive(Debug, Default)]
pub struct AddContainerState {}

pub fn container_view<Message>(container: DbContainer, image: Handle) -> ContainerView<Message> {
    ContainerView::new(container, image)
}

impl<Message> ContainerView<Message> {
    pub fn new(container: DbContainer, image: Handle) -> Self {
        Self {
            container,
            image,
            t: PhantomData,
        }
    }
}

impl<Message> Component<Message, Renderer> for ContainerView<Message> {
    type State = AddContainerState;

    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {}
    }

    fn view(&self, _state: &Self::State) -> iced_aw::Element<'_, Self::Event, Renderer> {
        let content = column!(row!(
            Image::new(self.image.clone()).height(35),
            text(
                self.container
                    .name
                    .strip_prefix('/')
                    .unwrap_or(&self.container.name)
            )
            .size(22)
        )
        .align_items(iced::Alignment::Center))
        .align_items(iced::Alignment::Center)
        .spacing(15)
        .padding(15);

        return scrollable(content).into();
    }
}

impl<'a, Message> From<ContainerView<Message>> for Element<'a, Message, Renderer>
where
    Message: 'a,
{
    fn from(value: ContainerView<Message>) -> Self {
        component(value)
    }
}
