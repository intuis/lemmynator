use std::{
    fs::File,
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use lemmy_api_common::{
    lemmy_db_schema::sensitive::SensitiveString,
    person::{Login, LoginResponse},
};
use ln_config::Config;
use ratatui_image::picker::Picker;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};

use crate::{
    action::{event_to_action, Action, Mode, UpdateAction},
    tui::Tui,
    ui::{components::Component, main_ui::MainWindow},
};

use anyhow::Result;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub struct App {
    should_quit: bool,
    action_rx: UnboundedReceiver<Action>,
    update_rx: UnboundedReceiver<UpdateAction>,
    main_window: MainWindow,
    mode: Mode,
    ctx: Arc<Ctx>,
}

pub struct Ctx {
    pub action_tx: UnboundedSender<Action>,
    pub update_tx: UnboundedSender<UpdateAction>,
    pub client: Client,
    pub picker: Mutex<Picker>,
    pub config: Config,
}

impl Ctx {
    pub fn send_action(&self, action: Action) {
        self.action_tx.send(action).unwrap();
    }

    pub fn send_update_action(&self, action: UpdateAction) {
        self.update_tx.send(action).unwrap();
    }
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (update_tx, update_rx) = mpsc::unbounded_channel();
        let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

        let client = Client::builder().user_agent(user_agent).build()?;

        let login_req = Login {
            username_or_email: SensitiveString::from(config.connection.username.clone()),
            password: SensitiveString::from(config.connection.password.clone()),
            ..Default::default()
        };

        let xdg_dirs = Config::get_xdg_dirs();
        let jwt_file = xdg_dirs.get_cache_file("jwt");

        let jwt = if jwt_file.exists() {
            let mut buf = String::new();
            File::open(jwt_file)
                .unwrap()
                .read_to_string(&mut buf)
                .unwrap();
            buf
        } else {
            let res: LoginResponse = client
                .post(format!(
                    "https://{}/api/v3/user/login",
                    config.connection.instance
                ))
                .json(&login_req)
                .send()
                .await?
                .json()
                .await?;
            let jwt = res.jwt.unwrap().to_string();
            File::create(xdg_dirs.place_cache_file("jwt").unwrap())
                .unwrap()
                .write(jwt.as_bytes())
                .unwrap();
            jwt
        };

        let mut header_map = HeaderMap::new();
        header_map.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", jwt))?,
        );
        let client = Client::builder()
            .user_agent(user_agent)
            .default_headers(header_map)
            .build()?;

        let picker = Picker::from_query_stdio().unwrap();

        let ctx = Arc::new(Ctx {
            action_tx,
            client,
            picker: Mutex::new(picker),
            config,
            update_tx,
        });

        Ok(Self {
            should_quit: false,
            main_window: MainWindow::new(Arc::clone(&ctx)).await?,
            action_rx,
            mode: Mode::Normal,
            update_rx,
            ctx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?;

        tui.enter()?;

        self.render(&mut tui)?;
        self.main_loop(&mut tui).await?;

        tui.exit()?;
        Ok(())
    }

    async fn main_loop(&mut self, tui: &mut Tui) -> Result<()> {
        loop {
            let tui_event = tui.next();
            let action = self.action_rx.recv();
            let update_action = self.update_rx.recv();

            tokio::select! {
                event = tui_event => {
                    if let Some(action) = event_to_action(self.mode, event.unwrap()) {
                        self.handle_action(action);
                    };
                },

                action = action => {
                    if let Some(action) = action {
                        if action.is_render() {
                            self.render(tui)?;
                        } else {
                            self.handle_action(action);
                        }
                    }
                },

                update_action = update_action => {
                    if let Some(update_action) = update_action {
                        self.handle_update_action(update_action);
                    }
                }
            }

            if self.should_quit {
                break Ok(());
            }
        }
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.terminal.draw(|f| {
            self.main_window.render(f, f.area());
        })?;
        Ok(())
    }

    fn handle_action(&mut self, action: Action) {
        match &action {
            Action::ForceQuit => {
                self.should_quit = true;
            }

            Action::SwitchToInputMode => {
                self.mode = Mode::Input;
                self.ctx.send_action(Action::Render);
            }

            Action::SwitchToNormalMode => {
                self.mode = Mode::Normal;
                self.ctx.send_action(Action::Render);
            }

            _ => {
                self.main_window.handle_actions(action);
            }
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        self.main_window.handle_update_action(action);
    }
}
