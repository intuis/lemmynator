use std::{rc::Rc, sync::Arc};

use crate::{
    action::Action,
    app::{Ctx, UserInfo},
};

use super::{
    components::{tabs::TabComponent, Component},
    page::Page,
};

use lemmy_api_common::lemmy_db_schema::ListingType;
use ratatui::prelude::*;

pub struct MainWindow {
    tabs: TabComponent,
    posts_viewer: PostsComponent,
    ctx: Arc<Ctx>,
}

impl MainWindow {
    pub async fn new(user_info: Rc<UserInfo>, ctx: Arc<Ctx>) -> Self {
        Self {
            tabs: TabComponent::new(),
            posts_viewer: PostsComponent::new(user_info, Arc::clone(&ctx)).await,
            ctx: Arc::clone(&ctx),
        }
    }
}

impl Component for MainWindow {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        return self.posts_viewer.handle_actions(action);
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [tabs_rect, main_rect] =
            Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]).areas(rect);

        let posts_rect = Layout::horizontal([
            Constraint::Percentage(5),
            Constraint::Percentage(90),
            Constraint::Percentage(5),
        ])
        .split(main_rect)[1];

        self.tabs.render(f, tabs_rect);
        self.posts_viewer.render(f, posts_rect);
    }
}

struct PostsComponent {
    user_info: Rc<UserInfo>,
    subscribed_page: Page,
    local_page: Page,
    all_page: Page,
    ctx: Arc<Ctx>,
}

impl PostsComponent {
    async fn new(user_info: Rc<UserInfo>, ctx: Arc<Ctx>) -> Self {
        Self {
            user_info,
            subscribed_page: Page::new(ListingType::Subscribed, Arc::clone(&ctx)).await,
            local_page: Page::new(ListingType::Local, Arc::clone(&ctx)).await,
            all_page: Page::new(ListingType::All, Arc::clone(&ctx)).await,
            ctx,
        }
    }
}

impl Component for PostsComponent {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        return self.local_page.handle_actions(action);
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        self.local_page.render(f, rect);
    }
}
