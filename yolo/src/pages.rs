use terminal::*;
use widgets2::*;

use crate::app::Action;

pub struct Pages {
    route: Route,
    demo: DemoPage,
    list: ListPage,
    tags: TagsPage,
    image: ImagePage,
    editor: EditorPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    Demo,
    List,
    Tags,
    Image,
    Editor,
}

impl Route {
    const DEFAULT: Self = Self::Demo;

    const fn next(self) -> Self {
        match self {
            Self::Demo => Self::List,
            Self::List => Self::Tags,
            Self::Tags => Self::Image,
            Self::Image => Self::Editor,
            Self::Editor => Self::Demo,
        }
    }

    const fn prev(self) -> Self {
        match self {
            Self::Demo => Self::Editor,
            Self::List => Self::Demo,
            Self::Tags => Self::List,
            Self::Image => Self::Tags,
            Self::Editor => Self::Image,
        }
    }
}

impl Pages {
    pub fn new(kitty: &mut KittyGraphics) -> Self {
        Self {
            route: Route::DEFAULT,
            demo: DemoPage::new(),
            list: ListPage::new(),
            tags: TagsPage::new(),
            image: ImagePage::new(kitty),
            editor: EditorPage::new(),
        }
    }

    pub fn forward(&mut self, frame: &mut Framebuffer) {
        self.set_route(self.route.next(), frame);
    }

    pub fn backward(&mut self, frame: &mut Framebuffer) {
        self.set_route(self.route.prev(), frame);
    }

    fn set_route(&mut self, route: Route, frame: &mut Framebuffer) {
        self.on_exit(self.route, frame);
        self.route = route;
        self.on_enter(route, frame);
    }

    fn on_enter(&mut self, route: Route, frame: &mut Framebuffer) {
        match route {
            Route::Demo => self.demo.on_enter(),
            Route::List => self.list.on_enter(),
            Route::Tags => self.tags.on_enter(),
            Route::Image => self.image.on_enter(),
            Route::Editor => self.editor.on_enter(frame),
        }
    }

    fn on_exit(&mut self, route: Route, frame: &mut Framebuffer) {
        match route {
            Route::Demo => self.demo.on_exit(),
            Route::List => self.list.on_exit(),
            Route::Tags => self.tags.on_exit(),
            Route::Image => self.image.on_exit(),
            Route::Editor => self.editor.on_exit(frame),
        }
    }

    pub fn on_render(&mut self, area: Rect, frame: &mut Framebuffer, kitty: &KittyGraphics) {
        match self.route {
            Route::Demo => self.demo.render(area, frame),
            Route::List => self.list.render(area, frame),
            Route::Tags => self.tags.render(area, frame),
            Route::Image => self.image.render(area, frame, kitty),
            Route::Editor => self.editor.render(area, frame),
        }
    }

    pub fn on_input(&mut self, key: KeyEvent) -> Action {
        match self.route {
            Route::Demo => self.demo.input(key),
            Route::List => self.list.input(key),
            Route::Tags => self.tags.input(key),
            Route::Image => self.image.input(key),
            Route::Editor => self.editor.input(key),
        }
    }

    pub fn render_navigation(&self, area: Rect, frame: &mut Framebuffer) {
        frame.push_str("TODO TOP");
        frame.render(area, TextOptions::span_center());
    }
}

struct DemoPage;

impl DemoPage {
    const fn new() -> Self {
        Self
    }

    fn on_enter(&self) {}

    fn on_exit(&self) {}

    fn render(&self, area: Rect, frame: &mut Framebuffer) {
        frame.cursor_move((2, 3));
        frame.print_str("Hello from my TUI!");

        frame.cursor_move((2, 4));
        frame.print_fmt(format_args!("Press {} to quit.", 'q'));

        frame.cursor_move((2, 6));
        frame.print_str("----------------------------------------------");
        frame.cursor_move((10, 6));
        frame.print_str(" yolo ");

        frame.cursor_move((10, 8));
        frame.print_fmt(Style::bg(Color::Red).text(" "));

        let center = (area.size.cols / 2, area.size.rows / 2);
        frame.cursor_move(center);
        frame.print_str("X");

        frame.cursor_move((center.0, center.1 + 2));
        frame.print_fmt(Styled::new("yoyoyo", Style::fg(Color::Red)));
        frame.print_str("_👻_yo?");

        frame.push_str("here is some ");
        frame.push_fmt(Styled::new("bold", Style::bold()));
        frame.push_str(" and ");
        frame.push_fmt(Styled::new("yellow", Style::fg(Color::Yellow)));
        frame.push_str(" text that wraps around oh yeah all is good indeed.\n");
        frame.push_str("More text incoming that also wraps again because why not.");
        let text_area = Rect {
            pos: Pos::new(2, area.size.rows.saturating_sub(10)),
            size: Size { cols: 7, rows: 11 },
        };
        frame.render(text_area.inner(Margin::all(1)), TextOptions::paragraph());
        Block::new(Shape::Rectangle)
            .with_color(Color::Cyan)
            .render(text_area, frame);

        let mut shape_area = Rect {
            pos: Pos::new(10, 10),
            size: Size::new(6, 3),
        };
        for shape in [
            Shape::LineHorizontal,
            Shape::LineVertical,
            Shape::Horizontals,
            Shape::Verticals,
            Shape::Rectangle,
            Shape::Corners,
        ] {
            Block::new(shape)
                .with_color(Color::Yellow)
                .render(shape_area, frame);
            shape_area.pos.col += shape_area.size.cols + 2;
        }

        let horz_area = area.with_col(area.cols() / 4).with_size(area.size / 6);
        Block::rectangle().render(horz_area, frame);
        for a in horz_area.split_horizontal(
            1,
            [
                Constraint::Percent(50),
                Constraint::Percent(50),
                Constraint::Fixed(3),
            ],
        ) {
            Block::rectangle().with_color(Color::Cyan).render(a, frame);
        }

        let vert_area = horz_area
            .with_row(area.rows() / 2)
            .with_rows(horz_area.rows() * 3);
        Block::rectangle().render(vert_area, frame);
        for a in vert_area.split_vertical(
            1,
            [
                Constraint::Percent(50),
                Constraint::Percent(50),
                Constraint::Fixed(3),
            ],
        ) {
            Block::rectangle().with_color(Color::Cyan).render(a, frame);
        }
    }

    fn input(&self, _key: KeyEvent) -> Action {
        Action::None
    }
}

struct ListPage {
    list: List,
    items: Vec<Yolo>,
}

struct Yolo;
impl Yolo {
    const fn yolo(&self) -> &'static str {
        "Yolo here be dragons"
    }
    const fn yolo2(&self) -> &'static str {
        "Another one"
    }
    const fn yolo3(&self) -> &'static str {
        "#Just do it"
    }
}

impl ListPage {
    fn new() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            items: Vec::from_iter((0..200).map(|_| Yolo)),
        }
    }

    fn on_enter(&self) {}

    fn on_exit(&self) {}

    fn render(&mut self, area: Rect, frame: &mut Framebuffer) {
        let (list_area, table_area) = {
            area.with_size(area.size.mul_f32(0.8, 0.6))
                .center(area)
                .split_vertically_with_gap(1)
        };

        Block::rectangle().render(list_area, frame);
        Block::rectangle().render(table_area, frame);

        frame.push_str(" LIST ");
        frame.render(list_area, TextOptions::span_center());
        frame.push_str(" TABLE ");
        frame.render(table_area, TextOptions::span_center());

        let margin = Margin::all(1);

        self.list.render(
            list_area.inner(margin),
            frame,
            self.items.iter(),
            |line, frame, item, idx| {
                let reset = match idx {
                    ListIndex::Selected => {
                        frame.push_fmt(Style::fg(Color::Yellow).with_reverse());
                        true
                    }
                    ListIndex::Selection => {
                        frame.push_fmt(Style::fg(Color::Indexed(245)).with_reverse());
                        true
                    }
                    ListIndex::Normal => false,
                };

                frame.push_str(item.yolo());
                frame.push_str(item.yolo2());
                frame.push_str(item.yolo3());
                frame.render(
                    line,
                    TextOptions {
                        mode: TextMode::Span { fill: true },
                        align: HorizontalAlignment::Left,
                    },
                );

                if reset {
                    frame.print_fmt(Sgr::Reset);
                }
            },
        );

        let table_area = table_area.inner(margin);
        let gap = if table_area.cols() < 10 { 0 } else { 2 };
        self.list.render_table(
            table_area,
            frame,
            self.items.iter(),
            TableLayout::new(
                gap,
                [
                    Constraint::Percent(60),
                    Constraint::Percent(40),
                    Constraint::Fixed(1),
                ],
            ),
            |_line, frame, areas| {
                let [a, b, c] = areas;
                for (a, s) in [(a, "Column 1"), (b, "Column 2"), (c, "Column 3")] {
                    frame.push_str(s);
                    frame.render(a, TextOptions::span());
                }
            },
            |line, frame, areas, item, idx| {
                let reset = match idx {
                    ListIndex::Selected => {
                        frame.print_fmt(Style::fg(Color::Yellow).with_reverse());
                        true
                    }
                    ListIndex::Selection => {
                        frame.print_fmt(Style::fg(Color::Indexed(245)).with_reverse());
                        true
                    }
                    ListIndex::Normal => false,
                };

                let [a, b, c] = areas;
                frame.push_str(item.yolo());
                frame.render(a, TextOptions::span_fill());
                frame.print_ch_repeat(' ', b.col() - a.right_out());

                frame.push_str(item.yolo2());
                frame.render(b, TextOptions::span_fill());
                frame.print_ch_repeat(' ', c.col() - b.right_out());

                frame.push_str(item.yolo3());
                frame.render(c, TextOptions::span_fill());
                frame.print_ch_repeat(' ', line.right_out() - c.right_out());

                if reset {
                    frame.print_fmt(Sgr::Reset);
                }
            },
        );
    }

    fn input(&mut self, key: KeyEvent) -> Action {
        if self.list.input(key.code, key.modifiers) {
            return Action::Render;
        }

        Action::None
    }
}

struct TagsPage {
    list: TagList,
    tags: Vec<Tag>,
}

struct Tag(&'static str);
impl TagItem for &Tag {
    fn width(&self) -> u16 {
        utils::str_width(self.0)
    }
}

impl TagsPage {
    fn new() -> Self {
        Self {
            list: TagList::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            tags: vec![
                Tag("tag"),
                Tag("tag2"),
                Tag("tag3"),
                Tag("abigasslongtagname"),
                Tag("yolo"),
                Tag("rust"),
                Tag("image"),
                Tag("compsci"),
                Tag("anotherbigasslongtagnamehereyolo"),
                Tag("music"),
                Tag("quote"),
                Tag("joke"),
                Tag("math"),
                Tag("recipe"),
                Tag("gaming"),
                Tag("science"),
                Tag("#metoo"),
            ],
        }
    }

    fn on_enter(&self) {}

    fn on_exit(&self) {}

    fn render(&mut self, area: Rect, frame: &mut Framebuffer) {
        let tags_area = area.with_size(area.size / 2).center(area);
        Block::rectangle().render(tags_area, frame);
        frame.push_str(" TAGS ");
        frame.render(tags_area, TextOptions::span_center());

        self.list.render(
            tags_area.inner(Margin::all(1)),
            frame,
            self.tags.iter(),
            |tag_area, frame, tag, is_selected| {
                if is_selected {
                    frame.push_fmt(Sgr::Fg(Color::Yellow));
                    frame.push_str(tag.0);
                    frame.push_fmt(Sgr::reset_fg());
                    frame.render(tag_area, TextOptions::span());
                } else {
                    frame.push_str(tag.0);
                    frame.render(tag_area, TextOptions::span());
                }
            },
        );
    }

    fn input(&mut self, key: KeyEvent) -> Action {
        if self.list.input(key.code, self.tags.iter()) {
            return Action::Render;
        }

        Action::None
    }
}

struct ImagePage {
    image: Image,
}

impl ImagePage {
    fn new(kitty: &mut KittyGraphics) -> Self {
        let mut image = Image::new(1);
        image.load_from_path("meow.png", kitty).unwrap();

        Self { image }
    }

    fn on_enter(&self) {}

    fn on_exit(&self) {}

    fn render(&mut self, area: Rect, frame: &mut Framebuffer, kitty: &KittyGraphics) {
        self.image.render(
            area.with_size(area.size / 4).center(area),
            frame,
            &kitty,
            ImageOptions::fit_and_center(),
        );
    }

    fn input(&mut self, _key: KeyEvent) -> Action {
        Action::None
    }
}

struct EditorPage {
    state: EditorPageState,
    prompt: Prompt,
    editor: Editor,
    editor_pos: Pos,
}

enum EditorPageState {
    Prompt,
    Editor,
}

impl EditorPage {
    const fn new() -> Self {
        Self {
            state: EditorPageState::Prompt,
            prompt: Prompt::new().with_placeholder("Search..."),
            editor: Editor::new().with_placeholder("Content...").with_disabled(),
            editor_pos: Pos::ZERO,
        }
    }

    fn on_enter(&self, frame: &mut Framebuffer) {
        frame.cursor_show();
    }

    fn on_exit(&self, frame: &mut Framebuffer) {
        frame.cursor_hide();
    }

    fn render(&mut self, area: Rect, frame: &mut Framebuffer) {
        let (prompt_area, editor_area) = {
            let mut center = area.with_size(area.size / 2).center(area);
            let prompt = center.with_rows(3);
            center.shrink_down(4);
            (prompt, center)
        };

        let (prompt_color, editor_color) = match self.state {
            EditorPageState::Prompt => (Color::Yellow, Color::Default),
            EditorPageState::Editor => (Color::Default, Color::Yellow),
        };

        let margin = Margin::symmetric(2, 1);
        let prompt_inner = prompt_area.inner(margin);
        let editor_inner = editor_area.inner(margin);

        Block::rectangle()
            .with_color(prompt_color)
            .render(prompt_area, frame);
        frame.push_str(" PROMPT ");
        frame.render(prompt_area, TextOptions::span_center());
        self.prompt.render(prompt_inner, frame);

        Block::rectangle()
            .with_color(editor_color)
            .render(editor_area, frame);
        frame.push_str(" EDITOR ");
        frame.render(editor_area, TextOptions::span_center());
        self.editor.render(editor_inner, frame);
        self.editor_pos = editor_inner.pos;

        let cpos = self.get_cursor_pos(prompt_inner.pos, editor_inner.pos);
        frame.set_cursor_at_end(cpos);
    }

    fn input(&mut self, key: KeyEvent) -> Action {
        let render = match key.code {
            KeyCode::Up => {
                if let EditorPageState::Editor = self.state {
                    let cpos = self.editor.get_cursor_pos(self.editor_pos);
                    if cpos.row == self.editor_pos.row {
                        self.state = EditorPageState::Prompt;
                        self.editor.set_disabled(true);
                        self.prompt.set_disabled(false);
                        true
                    } else {
                        self.editor.input(key.code, key.modifiers)
                    }
                } else {
                    false
                }
            }
            KeyCode::Down => {
                if let EditorPageState::Prompt = self.state {
                    self.state = EditorPageState::Editor;
                    self.prompt.set_disabled(true);
                    self.editor.set_disabled(false);
                    true
                } else {
                    self.editor.input(key.code, key.modifiers)
                }
            }
            _ => match self.state {
                EditorPageState::Prompt => self.prompt.input(key.code, key.modifiers),
                EditorPageState::Editor => self.editor.input(key.code, key.modifiers),
            },
        };

        if render {
            return Action::Render;
        }

        Action::None
    }

    fn get_cursor_pos(&self, prompt_pos: Pos, editor_pos: Pos) -> Pos {
        match self.state {
            EditorPageState::Prompt => self.prompt.get_cursor_pos(prompt_pos),
            EditorPageState::Editor => self.editor.get_cursor_pos(editor_pos),
        }
    }
}
