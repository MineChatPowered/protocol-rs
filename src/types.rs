use kyori_component_json::Component;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Message content that can be either CommonMark text or rich Components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain CommonMark text content
    CommonMark(String),
    /// Rich Minecraft text component content  
    Components(Component),
}

impl MessageContent {
    /// Creates a CommonMark message content
    pub fn commonmark(text: impl Into<String>) -> Self {
        Self::CommonMark(text.into())
    }

    /// Creates a Components message content
    pub fn components(component: Component) -> Self {
        Self::Components(component)
    }

    /// Returns a plain text representation of this content
    pub fn to_plain_text(&self) -> Cow<'_, str> {
        match self {
            MessageContent::CommonMark(text) => Cow::Borrowed(text),
            MessageContent::Components(component) => {
                Cow::Owned(component.to_plain_text().into_owned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyori_component_json::{Color, Component, NamedColor};

    #[test]
    fn test_message_content_commonmark_serialization() {
        let content = MessageContent::CommonMark("Hello, world!".to_string());
        let serialized = serde_cbor::to_vec(&content).unwrap();
        let deserialized: MessageContent = serde_cbor::from_slice(&serialized).unwrap();
        assert!(matches!(deserialized, MessageContent::CommonMark(_)));
    }

    #[test]
    fn test_message_content_components_serialization() {
        let component = Component::text("Hello")
            .color(Some(Color::Named(NamedColor::Red)))
            .decoration(kyori_component_json::TextDecoration::Bold, Some(true));
        let content = MessageContent::components(component);

        let serialized = serde_cbor::to_vec(&content).unwrap();
        let deserialized: MessageContent = serde_cbor::from_slice(&serialized).unwrap();

        assert!(matches!(deserialized, MessageContent::Components(_)));
    }
}
