mod twitch_chat;

use thiserror::Error;
pub use twitch_chat::TwitchChatConnector;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("Receiving message failed: {0:?}")]
    MessageReceiveFailed(String),
    #[error("Sending message failed: {0:?}")]
    MessageSendFailed(String),
    #[error("Unknown command: {0:?}")]
    UnknownCommand(String),
}

#[derive(Debug)]
pub enum CommandType {
    Help,
    Info,
}

#[derive(Debug)]
pub struct Command {
    pub commmand_type: CommandType,
    pub options: Vec<String>,
    pub user_name: String,
}

impl Command {
    fn new(text: &str, user_name: &str) -> Option<Self> {
        if !text.starts_with('!') {
            None
        } else {
            let command_end_index = text.find(' ').unwrap_or(text.len() - 1);
            let command_text = &text[1..command_end_index];
            let options: Vec<String> = text[(command_end_index + 1)..]
                .split(' ')
                .map(String::from)
                .collect();
            match command_text {
                "help" => Some(Self {
                    commmand_type: CommandType::Help,
                    options: options,
                    user_name: user_name.to_owned(),
                }),
                "info" => Some(Self {
                    commmand_type: CommandType::Info,
                    options: options,
                    user_name: user_name.to_owned(),
                }),
                _ => None,
            }
        }
    }
}

#[derive(Debug)]
pub struct TextMessage {
    text: String,
    user_name: String,
}

// Example text: #channel_name :backseating backseating
impl TextMessage {
    fn new(text: &str, user_name: &str) -> Self {
        Self {
            text: text.to_owned(),
            user_name: user_name.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct Part(String);

impl Part {
    fn new(user_name: &str) -> Self {
        Self(user_name.to_owned())
    }
}

#[derive(Debug)]
pub struct Join(String);

impl Join {
    fn new(user_name: &str) -> Self {
        Self(user_name.to_owned())
    }
}

#[derive(Debug)]
pub enum EventContent {
    TextMessage(TextMessage),
    Command(Command),
    Part(Part),
    Join(Join),
}

// Example message: :carkhy!carkhy@carkhy.tmi.twitch.tv PRIVMSG #captaincallback :backseating backseating
impl EventContent {
    fn new(message: &str) -> Option<Self> {
        enum ParsingState {
            UserName,
            AdditionalUserInfo,
            MessageToken,
            Channel,
            MessageBody,
        }
        use ParsingState::*;

        let mut state = UserName;
        let mut user_name = &message[0..0];
        let mut marker = 0;

        for (i, codepoint) in message.char_indices() {
            match state {
                // :carkhy!carkhy@carkhy.tmi.twitch.tv
                UserName => match codepoint {
                    ':' => marker = i + 1,
                    ' ' => return None,
                    '!' => {
                        user_name = &message[marker..i];
                        state = AdditionalUserInfo;
                    }
                    _ => (),
                },
                AdditionalUserInfo => {
                    if codepoint == ' ' {
                        marker = i + 1;
                        state = MessageToken
                    }
                }
                MessageToken => {
                    if codepoint == ' ' {
                        let token = &message[marker..i];
                        match token {
                            // (...) PRIVMSG #<channel> :backseating backseating
                            "PRIVMSG" => {
                                state = Channel;
                            }
                            // (...) JOIN #<channel>
                            "JOIN" => return Some(EventContent::Join(Join::new(user_name))),
                            // (...) PART #<channel>
                            "PART" => return Some(EventContent::Part(Part::new(user_name))),
                            // PING :tmi.twitch.tv
                            _ => return None,
                        };
                    }
                }
                Channel => {
                    if codepoint == ':' {
                        state = MessageBody;
                    }
                }
                MessageBody => {
                    if codepoint == '!' {
                        return Some(EventContent::Command(Command::new(
                            &message[i..].trim(),
                            user_name,
                        )?));
                    } else {
                        return Some(EventContent::TextMessage(TextMessage::new(
                            &message[i..].trim(),
                            user_name,
                        )));
                    }
                }
            }
        }
        None
    }
}

pub trait Event {
    fn content(&self) -> &EventContent;
    fn respond(&mut self, response: &str) -> Result<(), ConnectorError>;
}

pub trait Connector {
    fn recv_event(&mut self) -> Result<Box<dyn Event + '_>, ConnectorError>;
}

#[cfg(test)]
mod tests {
    use crate::connect::EventContent;

    fn user_message_helper(raw_message: &str, user_name: &str, expected: &str) {
        let parsed = EventContent::new(raw_message);
        assert!(parsed.is_some());
        if let EventContent::TextMessage(user_message) = parsed.unwrap() {
            assert_eq!(user_message.user_name, user_name);
            assert_eq!(user_message.text, expected);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn parsing_user_messages() {
        let raw_message = ":carkhy!carkhy@carkhy.tmi.twitch.tv PRIVMSG #captaincallback :a function that takes a string and returns the message";
        let expected_text = "a function that takes a string and returns the message";
        let expected_user = "carkhy";
        user_message_helper(raw_message, expected_user, expected_text);
    }

    #[test]
    fn parsing_user_messages_with_trailing_newlines() {
        let raw_message = ":carkhy!carkhy@carkhy.tmi.twitch.tv PRIVMSG #captaincallback :a function that takes a string and returns the message\n";
        let expected_text = "a function that takes a string and returns the message";
        let expected_user = "carkhy";
        user_message_helper(raw_message, expected_user, expected_text);
    }
}