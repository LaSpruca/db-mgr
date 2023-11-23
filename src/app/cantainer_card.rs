use iced::{
    theme::{Button, Text},
    widget::{button, column, component, container, horizontal_rule, image, row, text, Component},
    Color, Element, Length, Pixels, Renderer,
};
use iced_aw::{Icon, ICON_FONT};

use crate::docker::DbContainer;

#[derive(Clone)]
pub enum Event {
    Start,
    Stop,
    View,
}

pub fn container_card<Message>(
    container: &DbContainer,
    thumbnail: image::Handle,
) -> ContainerCard<Message> {
    ContainerCard::new(container.clone(), thumbnail)
}

pub struct ContainerCard<Message> {
    container: DbContainer,
    on_start_click: Option<Box<dyn Fn(String) -> Message>>,
    on_stop_click: Option<Box<dyn Fn(String) -> Message>>,
    on_view_click: Option<Box<dyn Fn(String) -> Message>>,
    image: image::Handle,
}

impl<Message> ContainerCard<Message> {
    pub fn new(container: DbContainer, thumbnail: image::Handle) -> Self {
        Self {
            container,
            on_start_click: None,
            on_stop_click: None,
            on_view_click: None,
            image: thumbnail,
        }
    }

    pub fn on_start_click<Callback>(self, handler: Callback) -> Self
    where
        Callback: Fn(String) -> Message + 'static,
    {
        Self {
            on_start_click: Some(Box::new(handler)),
            ..self
        }
    }

    pub fn on_stop_click<Callback>(self, handler: Callback) -> Self
    where
        Callback: Fn(String) -> Message + 'static,
    {
        Self {
            on_stop_click: Some(Box::new(handler)),
            ..self
        }
    }

    pub fn on_view_click<Callback>(self, handler: Callback) -> Self
    where
        Callback: Fn(String) -> Message + 'static,
    {
        Self {
            on_view_click: Some(Box::new(handler)),
            ..self
        }
    }
}

impl<Message> Component<Message, Renderer> for ContainerCard<Message> {
    type State = ();

    type Event = Event;

    fn update(&mut self, _: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::Start => self
                .on_start_click
                .as_ref()
                .map(|fun| fun(self.container.id.clone())),
            Event::Stop => self
                .on_stop_click
                .as_ref()
                .map(|fun| fun(self.container.id.clone())),
            Event::View => self
                .on_view_click
                .as_ref()
                .map(|fun| fun(self.container.id.clone())),
        }
    }

    fn view(&self, _: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let mut buttons = row(vec![])
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .align_items(iced::Alignment::Center)
            .spacing(5);

        match self.container.state {
            bollard::service::ContainerStateStatusEnum::CREATED
            | bollard::service::ContainerStateStatusEnum::PAUSED
            | bollard::service::ContainerStateStatusEnum::EXITED => {
                buttons = buttons.push(
                    button(text(Icon::PlayFill).font(ICON_FONT))
                        .style(Button::Positive)
                        .on_press(Event::Start),
                )
            }
            bollard::service::ContainerStateStatusEnum::RUNNING => {
                buttons = buttons.push(
                    button(text(Icon::StopFill).font(ICON_FONT))
                        .style(Button::Destructive)
                        .on_press(Event::Stop),
                );
            }
            _ => {}
        };

        buttons = buttons.push(button("View").on_press(Event::View));

        column!(
            row!(
                container(image::Image::new(self.image.clone()).height(30))
                    .width(Length::FillPortion(1))
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Center)
                    .align_x(iced::alignment::Horizontal::Center),
                column!(
                    text(&self.container.name).size(20),
                    text(&self.container.image).style(Text::Color(Color::from_rgb8(150, 150, 150)))
                )
                .width(Length::FillPortion(3))
                .height(Length::Fill),
                buttons,
            ),
            horizontal_rule(2)
        )
        .width(Length::Fill)
        .height(Pixels(50.0f32))
        .into()
    }
}

impl<'a, Message> From<ContainerCard<Message>> for Element<'a, Message, Renderer>
where
    Message: 'a,
{
    fn from(value: ContainerCard<Message>) -> Self {
        component(value)
    }
}
