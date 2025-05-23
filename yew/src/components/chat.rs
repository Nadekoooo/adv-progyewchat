use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{
    User,
    services::{
        websocket::WebsocketService,
        event_bus::EventBus,
    },
    Route,
};

/// Messages handled by the Chat component
pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
    NoOp,
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
    messages: Vec<MessageData>,
    chat_input: NodeRef,
    wss: WebsocketService,
    _bus: Box<dyn Bridge<EventBus>>,
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // get current user from context
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("no User context");
        let username = user.username.borrow().clone();

        // setup websocket service and register user
        let wss = WebsocketService::new();
        let register_msg = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username),
            data_array: None,
        };
        let _ = wss.tx.clone()
            .try_send(serde_json::to_string(&register_msg).unwrap());

        // bridge event bus for incoming WS messages
        let bus_cb = ctx.link().callback(Msg::HandleMsg);
        let bus = EventBus::bridge(bus_cb);

        Chat {
            users: Vec::new(),
            messages: Vec::new(),
            chat_input: NodeRef::default(),
            wss,
            _bus: bus,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::NoOp => false,
            Msg::HandleMsg(raw) => {
                let wsm: WebSocketMessage = serde_json::from_str(&raw).unwrap();
                match wsm.message_type {
                    MsgTypes::Users => {
                        let list = wsm.data_array.unwrap_or_default();
                        self.users = list.into_iter().map(|u| {
                            UserProfile {
                                name: u.clone(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                ),
                            }
                        }).collect();
                        true
                    }
                    MsgTypes::Message => {
                        if let Some(payload) = wsm.data {
                            let md: MessageData = serde_json::from_str(&payload).unwrap();
                            self.messages.push(md);
                        }
                        true
                    }
                    _ => false,
                }
            }
            Msg::SubmitMessage => {
                if let Some(input) = self.chat_input.cast::<HtmlInputElement>() {
                    let text = input.value();
                    if !text.is_empty() {
                        let msg = WebSocketMessage {
                            message_type: MsgTypes::Message,
                            data: Some(text.clone()),
                            data_array: None,
                        };
                        let _ = self.wss.tx.clone()
                            .try_send(serde_json::to_string(&msg).unwrap());
                    }
                    input.set_value("");
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        // a simple set of emojis
        let emojis = ["😀", "😂", "😍", "👍", "🙏"];

        html! {
            <div class="flex w-screen h-screen">
                // Sidebar: Users list
                <div class="flex-none w-64 bg-gray-100 p-4 overflow-y-auto">
                    <h2 class="text-lg font-semibold mb-2">{"Users Online"}</h2>
                    { for self.users.iter().map(|u| html! {
                        <div class="flex items-center mb-3 p-2 bg-white rounded shadow-sm">
                            <img class="w-10 h-10 rounded-full mr-3" src={u.avatar.clone()} alt="avatar"/>
                            <div>
                                <div class="font-medium">{ &u.name }</div>
                            </div>
                        </div>
                    }) }
                </div>

                // Main chat area
                <div class="flex-grow flex flex-col">
                    // Chat header
                    <div class="h-16 flex items-center px-4 bg-white border-b">
                        <h1 class="text-xl font-bold">{"💬 YewChat"}</h1>
                    </div>

                    // Messages viewport
                    <div class="flex-grow overflow-auto p-4 bg-gray-50">
                        { for self.messages.iter().map(|m| {
                            let avatar = self.users
                                .iter()
                                .find(|u| u.name == m.from)
                                .map(|u| u.avatar.clone())
                                .unwrap_or_else(|| {
                                    "https://avatars.dicebear.com/api/adventurer-neutral/unknown.svg"
                                        .to_string()
                                });
                            html! {
                                <div class="flex items-start mb-4">
                                    <img class="w-8 h-8 rounded-full mr-2" src={avatar.clone()} alt="avatar"/>
                                    <div class="bg-white rounded-lg shadow p-2 max-w-md">
                                        <div class="text-sm font-semibold text-blue-600">{ &m.from }</div>
                                        <div class="mt-1 text-gray-800">
                                            {
                                                if m.message.ends_with(".gif") {
                                                    html! { <img class="rounded" src={m.message.clone()} alt="gif"/> }
                                                } else {
                                                    html! { { &m.message } }
                                                }
                                            }
                                        </div>
                                    </div>
                                </div>
                            }
                        }) }
                    </div>

                    // Emoji picker
                    <div class="flex space-x-2 p-2 bg-gray-100 border-t">
                        {
                            for emojis.iter().map(|&e| {
                                let chat_input = self.chat_input.clone();
                                let onemoji = ctx.link().callback(move |_| {
                                    if let Some(input) = chat_input.cast::<HtmlInputElement>() {
                                        let mut v = input.value();
                                        v.push_str(e);
                                        input.set_value(&v);
                                    }
                                    Msg::NoOp
                                });
                                html! {
                                    <button onclick={onemoji} class="text-2xl leading-none">{ e }</button>
                                }
                            })
                        }
                    </div>

                    // Input & send
                    <div class="h-14 flex items-center px-4 bg-white border-t">
                        <input
                            ref={self.chat_input.clone()}
                            type="text"
                            placeholder="Type a message…"
                            class="flex-1 px-4 py-2 border rounded-full mr-2 focus:outline-none"
                            onkeypress={ctx.link().callback(|e: KeyboardEvent| {
                                if e.key() == "Enter" { Msg::SubmitMessage } else { Msg::NoOp }
                            })}
                        />
                        <button onclick={submit}
                                class="px-4 py-2 bg-blue-600 text-white rounded-full">
                            {"Send"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}