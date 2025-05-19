use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{User, services::websocket::WebsocketService};
use crate::services::event_bus::EventBus;

use std::collections::HashMap;
use web_sys::HtmlSelectElement;

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
    ChangeTheme(Theme),
    ToggleEmojiPicker,
    AddEmoji(String),
    AddReaction(usize, String),
}

#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    wss: WebsocketService,
    messages: Vec<MessageData>,
    _producer: Box<dyn Bridge<EventBus>>,
    current_theme: Theme,
    show_emoji_picker: bool,
    message_reactions: HashMap<usize, HashMap<String, usize>>
}

#[derive(Clone, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    Ocean,
    Forest,
}

impl Theme {
    fn get_css_classes(&self) -> &str {
        match self {
            Theme::Light => "bg-white text-black",
            Theme::Dark => "bg-gray-800 text-black",
            Theme::Ocean => "bg-blue-900 text-black",
            Theme::Forest => "bg-green-900 text-black",
        }
    }
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();
    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let message = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.to_string()),
            data_array: None,
        };

        if let Ok(_) = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&message).unwrap())
        {
            log::debug!("message sent successfully");
        }

        Self {
            users: vec![],
            messages: vec![],
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
            current_theme: Theme::Dark,
            show_emoji_picker: false,
            message_reactions: HashMap::new(),
        }
    }
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                let msg: WebSocketMessage = serde_json::from_str(&s).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let users_from_message = msg.data_array.unwrap_or_default();
                        self.users = users_from_message
                            .iter()
                            .map(|u| UserProfile {
                                name: u.into(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                )
                                .into(),
                            })
                            .collect();
                        return true;
                    }
                    MsgTypes::Message => {
                        let message_data: MessageData =
                            serde_json::from_str(&msg.data.unwrap()).unwrap();
                        self.messages.push(message_data);
                        return true;
                    }
                    _ => {
                        return false;
                    }
                }
            }
            Msg::SubmitMessage => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    //log::debug!("got input: {:?}", input.value());
                    let message = WebSocketMessage {
                        message_type: MsgTypes::Message,
                        data: Some(input.value()),
                        data_array: None,
                    };
                    if let Err(e) = self
                        .wss
                        .tx
                        .clone()
                        .try_send(serde_json::to_string(&message).unwrap())
                    {
                        log::debug!("error sending to channel: {:?}", e);
                    }
                    input.set_value("");
                };
                false
            }
            Msg::ChangeTheme(theme) => {
                self.current_theme = theme;
                true
            },
            
            Msg::ToggleEmojiPicker => {
                self.show_emoji_picker = !self.show_emoji_picker;
                true
            },
            
            Msg::AddEmoji(emoji) => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    let current_value = input.value();
                    input.set_value(&format!("{} {}", current_value, emoji));
                    self.show_emoji_picker = false;
                }
                true
            },
            
            Msg::AddReaction(msg_idx, emoji) => {
                let reactions = self.message_reactions.entry(msg_idx).or_insert_with(HashMap::new);
                let count = reactions.entry(emoji).or_insert(0);
                *count += 1;
                true
            },
        }
    }
    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        let toggle_emoji = ctx.link().callback(|_| Msg::ToggleEmojiPicker);
        
        let theme_callback = ctx.link().callback(|e: Event| {
            let select = e.target_dyn_into::<HtmlSelectElement>().unwrap();
            let theme = match select.value().as_str() {
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                "ocean" => Theme::Ocean,
                "forest" => Theme::Forest,
                _ => Theme::Dark,
            };
            Msg::ChangeTheme(theme)
        });

        let theme_classes = self.current_theme.get_css_classes();

        let mut current_user = String::new();
        let mut message_index = 0;

        html! {
            <div class={format!("flex w-screen {}", theme_classes)}>
                <div class="flex-none w-56 h-screen bg-opacity-90 bg-gray-100">
                    <div class="p-3 flex justify-between items-center">
                        <div class="text-xl">{"Users"}</div>
                        <select onchange={theme_callback} class="px-2 py-1 rounded bg-white">
                            <option value="light" selected={self.current_theme == Theme::Light}>{"☀️ Light"}</option>
                            <option value="dark" selected={self.current_theme == Theme::Dark}>{"🌙 Dark"}</option>
                            <option value="ocean" selected={self.current_theme == Theme::Ocean}>{"🌊 Ocean"}</option>
                            <option value="forest" selected={self.current_theme == Theme::Forest}>{"🌲 Forest"}</option>
                        </select>
                    </div>
                    
                    <div class="overflow-y-auto max-h-[calc(100vh-80px)]">
                        {
                            self.users.clone().iter().map(|u| {
                                html!{
                                    <div class="flex m-3 bg-white rounded-lg p-2 shadow-sm hover:shadow-md transition-shadow duration-200">
                                        <div>
                                            <img class="w-12 h-12 rounded-full border-2 border-gray-200" src={u.avatar.clone()} alt="avatar"/>
                                        </div>
                                        <div class="flex-grow p-3">
                                            <div class="flex text-xs justify-between font-bold">
                                                <div>{u.name.clone()}</div>
                                            </div>
                                            <div class="text-xs text-gray-400">
                                                {"Online"}
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                </div>
                
                <div class="grow h-screen flex flex-col">
                    <div class="w-full h-14 border-b-2 border-gray-300 flex items-center justify-between px-4">
                        <div class="text-xl font-bold">{"💬 Chat Room"}</div>
                        <div class="text-sm text-gray-500">{format!("{} Active Users", self.users.len())}</div>
                    </div>
                    
                    <div class="w-full grow overflow-auto border-b-2 border-gray-300 p-4">
                        {
                            self.messages.iter().map(|m| {
                                let user_profile = self.users.iter()
                                    .find(|u| u.name == m.from)
                                    .cloned()
                                    .unwrap_or_else(|| UserProfile {
                                        name: m.from.clone(),
                                        avatar: format!("https://avatars.dicebear.com/api/adventurer-neutral/{}.svg", m.from)
                                    });
                                
                                let is_new_user = current_user != m.from;
                                current_user = m.from.clone();
                                
                                let msg_idx = message_index;
                                message_index += 1;
                                
                                let reactions = self.message_reactions.get(&msg_idx).cloned().unwrap_or_default();
                                
                                let add_reaction = ctx.link().callback(move |emoji: String| {
                                    Msg::AddReaction(msg_idx, emoji)
                                });
                                
                                html!{
                                    <div class={if is_new_user { "mt-6" } else { "mt-1" }}>
                                        if is_new_user {
                                            <div class="flex items-center mb-1">
                                                <img class="w-8 h-8 rounded-full mr-2" src={user_profile.avatar.clone()} alt="avatar"/>
                                                <div class="font-medium">{user_profile.name.clone()}</div>
                                            </div>
                                        }
                                        <div class={format!("flex flex-col ml-{}", if is_new_user { "0" } else { "10" })}>
                                            <div class="max-w-3/4 bg-gray-100 p-3 rounded-lg shadow-sm">
                                                if m.message.ends_with(".gif") {
                                                    <img class="max-h-64 rounded" src={m.message.clone()}/>
                                                } else {
                                                    <div class="text-sm whitespace-pre-wrap break-words">
                                                        {m.message.clone()}
                                                    </div>
                                                }
                                            </div>
                                            
                                            if !reactions.is_empty() {
                                                <div class="flex mt-1 ml-2 flex-wrap">
                                                    {
                                                        reactions.iter().map(|(emoji, count)| {
                                                            let emoji_clone = emoji.clone();
                                                            html! {
                                                                <button 
                                                                    onclick={add_reaction.reform(move |_| emoji_clone.clone())}
                                                                    class="bg-gray-200 rounded-full px-2 py-1 text-xs mr-1 mb-1"
                                                                >
                                                                    {format!("{} {}", emoji, count)}
                                                                </button>
                                                            }
                                                        }).collect::<Html>()
                                                    }
                                                </div>
                                            }
                                            
                                            <div class="flex mt-1 ml-2">
                                                <button 
                                                    onclick={add_reaction.reform(move |_| "👍".to_string())}
                                                    class="text-gray-500 hover:text-gray-700 text-xs mr-2"
                                                >
                                                    {"👍"}
                                                </button>
                                                <button 
                                                    onclick={add_reaction.reform(move |_| "❤️".to_string())}
                                                    class="text-gray-500 hover:text-gray-700 text-xs mr-2"
                                                >
                                                    {"❤️"}
                                                </button>
                                                <button 
                                                    onclick={add_reaction.reform(move |_| "😂".to_string())}
                                                    class="text-gray-500 hover:text-gray-700 text-xs mr-2"
                                                >
                                                    {"😂"}
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    
                    <div class="w-full flex flex-col px-3 py-2 relative">
                        if self.show_emoji_picker {
                            <div class="absolute bottom-16 right-5 bg-white shadow-lg rounded-lg p-2 w-64 h-48 overflow-auto">
                                <div class="grid grid-cols-8 gap-1">
                                    {
                                        ["😀", "😂", "😊", "🥰", "😍", "😎", "🙄", "😴", 
                                        "🤔", "🤯", "😱", "🥳", "😭", "😡", "🤢", "👍",
                                        "👎", "👏", "🙏", "💪", "🤝", "❤️", "💔", "💯",
                                        "🔥", "💩", "🎉", "✨", "🌈", "⭐", "🎁", "🏆"]
                                            .iter()
                                            .map(|emoji| {
                                                let emoji_str = emoji.to_string();
                                                let emoji_callback = ctx.link().callback(move |_| {
                                                    Msg::AddEmoji(emoji_str.clone())
                                                });
                                                html! {
                                                    <button 
                                                        onclick={emoji_callback} 
                                                        class="text-2xl hover:bg-gray-100 rounded p-1"
                                                    >
                                                        {*emoji}
                                                    </button>
                                                }
                                            })
                                            .collect::<Html>()
                                    }
                                </div>
                            </div>
                        }
                        
                        <div class="flex items-center">
                            <input 
                                ref={self.chat_input.clone()} 
                                type="text" 
                                placeholder="Type a message..." 
                                class="block w-full py-2 pl-4 mx-3 bg-gray-100 rounded-full outline-none focus:ring-2 focus:ring-blue-600" 
                                name="message" 
                                required=true 
                            />
                            <button 
                                onclick={toggle_emoji} 
                                class="p-3 bg-gray-200 rounded-full flex justify-center items-center mr-2 hover:bg-gray-300"
                            >
                                {"😊"}
                            </button>
                            <button 
                                onclick={submit} 
                                class="p-3 shadow-sm bg-blue-600 w-10 h-10 rounded-full flex justify-center items-center hover:bg-blue-700 transition-colors duration-200"
                            >
                                <svg fill="#000000" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="fill-white w-5 h-5">
                                    <path d="M0 0h24v24H0z" fill="none"></path><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        }
    }
}