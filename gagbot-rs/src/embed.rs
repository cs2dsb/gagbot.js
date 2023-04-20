use std::sync::Arc;

use poise::{
    serenity_prelude::{self as serenity, Http, Message, CreateEmbed},
    ChoiceParameter, ReplyHandle,
};
use serenity::Color;

use crate::{
    ChannelId, Context, GAGBOT_COLOR_ERROR, GAGBOT_COLOR_LOG_DELETE, GAGBOT_COLOR_LOG_EDIT,
    GAGBOT_COLOR_NORMAL, GAGBOT_COLOR_SUCCESS, GAGBOT_ICON, GAGBOT_ICON_ERROR, GAGBOT_COLOR_LOG_JOIN,
    GAGBOT_COLOR_LOG_LEAVE
};

#[derive(Clone, Copy, ChoiceParameter, PartialEq)]
pub enum EmbedFlavour {
    Normal,
    Error,
    Success,
    LogEdit,
    LogDelete,
    LogJoin,
    LogLeave,
}

impl Into<Color> for EmbedFlavour {
    fn into(self) -> Color {
        match self {
            EmbedFlavour::Normal => GAGBOT_COLOR_NORMAL,
            EmbedFlavour::Error => GAGBOT_COLOR_ERROR,
            EmbedFlavour::Success => GAGBOT_COLOR_SUCCESS,
            EmbedFlavour::LogEdit => GAGBOT_COLOR_LOG_EDIT,
            EmbedFlavour::LogDelete => GAGBOT_COLOR_LOG_DELETE,
            EmbedFlavour::LogJoin => GAGBOT_COLOR_LOG_JOIN,
            EmbedFlavour::LogLeave => GAGBOT_COLOR_LOG_LEAVE,
        }
        .into()
    }
}

impl EmbedFlavour {
    pub fn thumbnail_url(self) -> &'static str {
        match self {
            EmbedFlavour::Error => GAGBOT_ICON_ERROR,
            _ => GAGBOT_ICON,
        }
        .into()
    }
}

#[derive(Default)]
pub struct Embed {
    pub color: Option<Color>,
    pub flavour: Option<EmbedFlavour>,
    pub thumbnail_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub footer: Option<String>,
    pub ephemeral: Option<bool>,
}

impl Embed {
    pub fn error() -> Self {
        Self {
            flavour: Some(EmbedFlavour::Error),
            title: Some("Error".to_string()),
            ..Default::default()
        }
    }
    pub fn success() -> Self {
        Self {
            flavour: Some(EmbedFlavour::Success),
            ..Default::default()
        }
    }
    pub fn join() -> Self {
        Self {
            flavour: Some(EmbedFlavour::LogJoin),
            ..Default::default()
        }
    }
    pub fn leave() -> Self {
        Self {
            flavour: Some(EmbedFlavour::LogLeave),
            ..Default::default()
        }
    }
    pub fn color<T: Into<Color>>(mut self, color: T) -> Self {
        self.color = Some(color.into());
        self
    }
    pub fn flavour(mut self, flavour: EmbedFlavour) -> Self {
        self.flavour = Some(flavour);
        self
    }
    pub fn set_error(mut self, is_error: bool) -> Self {
        if is_error {
            self.flavour = Some(EmbedFlavour::Error);
        } else if self.flavour == Some(EmbedFlavour::Error) {
            self.flavour = None;
        }
        self
    }
    pub fn thumbnail_url<T: ToString>(mut self, thumbnail_url: T) -> Self {
        self.thumbnail_url = Some(thumbnail_url.to_string());
        self
    }
    pub fn title<T: ToString>(mut self, title: T) -> Self {
        self.title = Some(title.to_string());
        self
    }
    pub fn description<T: ToString>(mut self, description: T) -> Self {
        self.description = Some(description.to_string());
        self
    }
    pub fn footer<T: ToString>(mut self, footer: T) -> Self {
        self.footer = Some(footer.to_string());
        self
    }
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = Some(ephemeral);
        self
    }
    pub async fn send<'a>(self, ctx: &Context<'a>) -> Result<ReplyHandle<'a>, serenity::Error> {
        let ephemeral = self.ephemeral.unwrap_or(true);

        ctx.send(|b| b
            .embed(|b| self.create_embed(b))
            .ephemeral(ephemeral)
        )
        .await
    }
    pub async fn send_in_channel<'a>(
        self,
        channel_id: ChannelId,
        http: &'a Arc<Http>,
    ) -> Result<Message, serenity::Error> {
        

        channel_id
            .send_message(http, |b| {
                b.embed(|b| self.create_embed(b))
            })
            .await
    }
    pub fn create_embed<'a>(self, mut b: &mut CreateEmbed) -> &mut CreateEmbed {        
        let Self {
            color,
            flavour,
            thumbnail_url,
            title,
            description,
            footer,
            ..
        } = self;

        let flavour = flavour.unwrap_or(EmbedFlavour::Normal);
        let color = color.unwrap_or(flavour.into());

        
        b = b
            .color(color);

        if let Some(thumbnail_url) = thumbnail_url {
            b = b.thumbnail(thumbnail_url);
        }

        if let Some(title) = title {
            b = b.title(title);
        }
        if let Some(description) = description {
            b = b.description(description);
        }
        if let Some(footer) = footer {
            b = b.footer(|b| b
                .text(footer));
        }

        b               
    }


}
