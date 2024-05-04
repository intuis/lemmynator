use std::{
    io::{BufReader, Cursor, Read},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    action::Action,
    app::{Ctx, UserInfo},
};

use super::components::{tabs::TabComponent, Component};

use lemmy_api_common::{
    lemmy_db_schema::{ListingType, SortType},
    lemmy_db_views::structs::{PaginationCursor, PostView},
    post::{GetPosts, GetPostsResponse},
};
use ratatui::{
    prelude::*,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Paragraph, Wrap,
    },
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};

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
    subscribed_viewer: ListingViewer,
    local_viewer: ListingViewer,
    all_viewer: ListingViewer,
    ctx: Arc<Ctx>,
}

impl PostsComponent {
    async fn new(user_info: Rc<UserInfo>, ctx: Arc<Ctx>) -> Self {
        Self {
            user_info,
            subscribed_viewer: ListingViewer::new(ListingType::Subscribed, Arc::clone(&ctx)).await,
            local_viewer: ListingViewer::new(ListingType::Local, Arc::clone(&ctx)).await,
            all_viewer: ListingViewer::new(ListingType::All, Arc::clone(&ctx)).await,
            ctx,
        }
    }
}

impl Component for PostsComponent {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        return self.local_viewer.handle_actions(action);
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        self.local_viewer.render(f, rect);
    }
}

struct ListingViewer {
    listing_type: ListingType,
    page: Page,
    ctx: Arc<Ctx>,
}

impl ListingViewer {
    async fn new(listing_type: ListingType, ctx: Arc<Ctx>) -> Self {
        Self {
            page: Page::new(listing_type, Arc::clone(&ctx)).await,
            listing_type,
            ctx,
        }
    }
}

impl Component for ListingViewer {
    #[must_use]
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        self.page.handle_actions(action)
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        self.page.render(f, rect);
    }
}

struct Page {
    next_page: Arc<Mutex<PaginationCursor>>,
    posts: Arc<Mutex<Vec<LemmynatorPost>>>,
    posts_offset: usize,
    currently_focused: u8,
    currently_displaying: u8,
    can_fetch_new_pages: Arc<AtomicBool>,
    ctx: Arc<Ctx>,
}

impl Page {
    async fn new(listing_type: ListingType, ctx: Arc<Ctx>) -> Self {
        let local_posts_req = GetPosts {
            type_: Some(ListingType::Local),
            limit: Some(20),
            sort: Some(SortType::Hot),
            ..Default::default()
        };

        let page = ctx
            .client
            .get("https://slrpnk.net/api/v3/post/list")
            .json(&local_posts_req)
            .send()
            .await
            .unwrap();

        let page: GetPostsResponse = page.json().await.unwrap();

        let next_page = page.next_page.unwrap();

        let mut posts = vec![];
        for post in page.posts {
            let post = LemmynatorPost::from_lemmy_post(post, Arc::clone(&ctx)).await;
            posts.push(post);
        }

        Self {
            posts: Arc::new(Mutex::new(posts)),
            next_page: Arc::new(Mutex::new(next_page)),
            posts_offset: 0,
            currently_focused: 0,
            currently_displaying: 0,
            can_fetch_new_pages: Arc::new(AtomicBool::new(true)),
            ctx: Arc::clone(&ctx),
        }
    }

    async fn fetch_next_page(
        cursor: Arc<Mutex<PaginationCursor>>,
        posts: Arc<Mutex<Vec<LemmynatorPost>>>,
        atomic_lock: Arc<AtomicBool>,
        ctx: Arc<Ctx>,
    ) {
        // TODO: shoot an action of render from Ctx
        let posts_req = GetPosts {
            type_: Some(ListingType::Local),
            sort: Some(lemmy_api_common::lemmy_db_schema::SortType::Hot),
            page_cursor: Some(cursor.lock().unwrap().clone()),
            limit: Some(20),
            ..Default::default()
        };

        let req = ctx
            .client
            .get("http://slrpnk.net/api/v3/post/list")
            .query(&posts_req);

        let page: GetPostsResponse = req.send().await.unwrap().json().await.unwrap();

        let mut new_posts = vec![];

        for post in page.posts {
            let new_post = LemmynatorPost::from_lemmy_post(post, Arc::clone(&ctx)).await;
            new_posts.push(new_post);
        }

        posts.lock().unwrap().append(&mut new_posts);
        *cursor.lock().unwrap() = page.next_page.unwrap();
        atomic_lock.store(true, Ordering::SeqCst);
    }

    fn scroll_up(&mut self) {
        if self.currently_focused == 0 {
            self.posts_offset -= self.currently_displaying as usize;
            self.currently_focused = self.currently_displaying - 1;
        } else {
            self.currently_focused -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.currently_focused += 1;
        if self.currently_focused >= self.currently_displaying {
            self.posts_offset += self.currently_displaying as usize;
            self.currently_focused = 0;
        }
    }

    fn update_count_of_currently_displaying(&mut self, rect: Rect) {
        self.currently_displaying = (rect.height / 8) as u8;
    }
}

impl Component for Page {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Up => {
                self.scroll_up();
                Some(Action::Render)
            }
            Action::Down => {
                self.scroll_down();
                Some(Action::Render)
            }
            _ => None,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        self.update_count_of_currently_displaying(rect);
        let blocks_count = rect.height / 8;

        let layouts = Layout::vertical(vec![
            Constraint::Length(8);
            self.currently_displaying as usize
        ])
        .split(rect);

        let mut posts_lock = self.posts.lock().unwrap();
        for index in 0..blocks_count {
            let layout = layouts[index as usize];
            let post = {
                match posts_lock.get_mut(self.posts_offset + index as usize) {
                    Some(post) => post,
                    None => {
                        drop(posts_lock);
                        if let Ok(true) = self.can_fetch_new_pages.compare_exchange(
                            true,
                            false,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            tokio::task::spawn(Self::fetch_next_page(
                                Arc::clone(&self.next_page),
                                Arc::clone(&self.posts),
                                Arc::clone(&self.can_fetch_new_pages),
                                Arc::clone(&self.ctx),
                            ));
                        }
                        break;
                    }
                }
            };
            if self.currently_focused == index as u8 {
                post.is_focused = true;
            }
            post.render(f, layout);
        }
    }
}

struct LemmynatorPost {
    name: String,
    body: Option<String>,
    is_focused: bool,
    decoded_image: Arc<Mutex<Option<Box<dyn StatefulProtocol>>>>,
    embed_description: Option<String>,
    embed_url: Option<String>,
    author: String,
    community: String,
    downvotes: i64,
    comments: i64,
    upvotes: i64,
}

impl LemmynatorPost {
    fn header(&self) -> String {
        format!(" {} ", self.name)
    }

    fn footer_right(&self) -> String {
        format!(
            " c/{}  u/{}   {}  {}  󰆉 {} ",
            self.community, self.author, self.upvotes, self.downvotes, self.comments
        )
    }

    async fn fetch_image(
        url: String,
        image: Arc<Mutex<Option<Box<dyn StatefulProtocol>>>>,
        ctx: Arc<Ctx>,
    ) {
        // TODO: Send render Action via ctx
        let new_image = {
            let res = ctx.client.get(url).send().await.unwrap();
            Some(res.bytes().await.unwrap())
        };

        let new_image = if let Some(image) = new_image {
            let dyn_image_res = image::io::Reader::new(Cursor::new(image))
                .with_guessed_format()
                .unwrap()
                .decode();
            if let Ok(dyn_image) = dyn_image_res {
                Some(ctx.picker.lock().unwrap().new_resize_protocol(dyn_image))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(new_image) = new_image {
            *image.lock().unwrap() = Some(new_image);
        }
    }

    async fn from_lemmy_post(lemmy_post: PostView, ctx: Arc<Ctx>) -> Self {
        let decoded_image = Arc::new(Mutex::new(None));

        if let Some(url) = lemmy_post.post.thumbnail_url {
            tokio::task::spawn(Self::fetch_image(
                url.as_str().to_string(),
                Arc::clone(&decoded_image),
                Arc::clone(&ctx),
            ));
        }

        let embed_url = lemmy_post
            .post
            .url
            .and_then(|db_url| Some(db_url.to_string()));

        LemmynatorPost {
            name: lemmy_post.post.name,
            body: lemmy_post.post.body,
            community: lemmy_post.community.name,
            author: lemmy_post.creator.name,
            embed_description: lemmy_post.post.embed_description,
            embed_url,
            is_focused: false,
            decoded_image,
            upvotes: lemmy_post.counts.upvotes,
            downvotes: lemmy_post.counts.downvotes,
            comments: lemmy_post.counts.comments,
        }
    }
}

impl Component for LemmynatorPost {
    fn handle_actions(&mut self, _action: Action) -> Option<Action> {
        None
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let inner_rect = rect.inner(&Margin::new(1, 1));

        let border_style = {
            if self.is_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            }
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Title::from(self.header()).alignment(Alignment::Left))
            .title(
                Title::from(self.footer_right())
                    .alignment(Alignment::Right)
                    .position(Position::Bottom),
            );

        f.render_widget(block, rect);

        let [_, image_rect, _, mut text_rect] = Layout::horizontal([
            Constraint::Length(1),
            Constraint::Length(20),
            Constraint::Length(1),
            Constraint::Percentage(75),
        ])
        .areas(inner_rect);

        if let Some(image) = &mut *self.decoded_image.lock().unwrap() {
            let image_state = StatefulImage::new(None);
            f.render_stateful_widget(image_state, image_rect, image);
        } else {
            text_rect = inner_rect;
        }

        if let Some(body) = &self.body {
            let text = Paragraph::new(body.as_str()).wrap(Wrap { trim: false });
            f.render_widget(text, text_rect);
        } else if let Some(embed_desc) = &self.embed_description {
            let text = Paragraph::new(embed_desc.as_str()).wrap(Wrap { trim: false });
            f.render_widget(text, text_rect);
        }

        self.is_focused = false;
    }
}
