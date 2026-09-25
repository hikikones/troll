use terminal::*;
use widgets2::*;

fn main() -> std::io::Result<()> {
    let mut terminal = Terminal::enter_tui()?;
    let mut kitty = KittyGraphics::new();

    // List
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
    let mut list = List::new()
        .with_scrollbar(0)
        .with_padding(Margin::horizontal(1));
    let items = Vec::from_iter((0..200).map(|_| Yolo));

    // Tags
    let mut taglist = TagList::new()
        .with_scrollbar(0)
        .with_padding(Margin::horizontal(1));
    struct Tag(&'static str);
    impl TagItem for &Tag {
        fn width(&self) -> u16 {
            utils::str_width(self.0)
        }
    }
    let tags = [
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
    ];

    // Image
    let id = 1;
    let mut image = Image::new(id);
    image.load_from_path("meow.png", &mut kitty).unwrap();

    // Prompt
    let mut prompt = Prompt::new().with_placeholder("Search...");

    // Editor
    let mut editor = Editor::new().with_placeholder("Content...");
    editor.push_str("yoyo\there\tare\tsome\ttabs\nnew line down here wut");

    #[derive(Debug)]
    enum Page {
        Demo,
        List,
        Tags,
        Image,
        Prompt,
        Editor,
    }

    let mut page = Page::Demo;

    loop {
        terminal.render(|frame| {
            let area = frame.area();

            if area.size.is_less(2) {
                return Ok(());
            }

            frame.print_fmt(KittyDeleteAll);

            frame.cursor_move((0, 0));
            frame.print_fmt(format_args!("Page: {page:?}"));

            match page {
                Page::Demo => {
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
                Page::List => {
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

                    list.render(
                        list_area.inner(margin),
                        frame,
                        items.iter(),
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
                    list.render_table(
                        table_area,
                        frame,
                        items.iter(),
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
                Page::Tags => {
                    let tags_area = area.with_size(area.size / 2).center(area);
                    Block::rectangle().render(tags_area, frame);
                    frame.push_str(" TAGS ");
                    frame.render(tags_area, TextOptions::span_center());

                    taglist.render(
                        tags_area.inner(Margin::all(1)),
                        frame,
                        tags.iter(),
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
                Page::Image => {
                    image.render(
                        area.with_size(area.size / 4).center(area),
                        frame,
                        &kitty,
                        ImageOptions::fit_and_center(),
                    );
                }
                Page::Prompt => {
                    let prompt_size = (area.size / 2).with_rows(3);
                    let prompt_area = area.with_size(prompt_size).center(area);
                    Block::rectangle().render(prompt_area, frame);

                    frame.push_str(" PROMPT ");
                    frame.render(prompt_area, TextOptions::span_center());

                    prompt.render(prompt_area.inner(Margin::symmetric(2, 1)), frame);
                }
                Page::Editor => {
                    let editor_area = area.with_size(area.size / 2).center(area);
                    Block::rectangle().render(editor_area, frame);

                    frame.push_str(" EDITOR ");
                    frame.render(editor_area, TextOptions::span_center());

                    editor.render(editor_area.inner(Margin::symmetric(2, 1)), frame);
                }
            }

            Ok(())
        })?;

        // let timeout = std::time::Duration::from_millis(1000);
        // let Some(event) = Terminal::poll_event(timeout)? else {
        //     continue;
        // };

        let event = Terminal::read_event()?;

        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => break,
                KeyCode::Tab => {
                    page = match page {
                        Page::Demo => Page::List,
                        Page::List => Page::Tags,
                        Page::Tags => Page::Image,
                        Page::Image => {
                            terminal.frame().cursor_show();
                            Page::Prompt
                        }
                        Page::Prompt => Page::Editor,
                        Page::Editor => {
                            terminal.frame().cursor_hide();
                            Page::Demo
                        }
                    };
                }
                KeyCode::BackTab => {
                    page = match page {
                        Page::Demo => {
                            terminal.frame().cursor_show();
                            Page::Editor
                        }
                        Page::List => Page::Demo,
                        Page::Tags => Page::List,
                        Page::Image => Page::Tags,
                        Page::Prompt => {
                            terminal.frame().cursor_hide();
                            Page::Image
                        }
                        Page::Editor => Page::Prompt,
                    };
                }
                _ => match page {
                    Page::Demo => {}
                    Page::List => {
                        list.input(key.code, key.modifiers);
                    }
                    Page::Tags => {
                        taglist.input(key.code, tags.iter());
                    }
                    Page::Image => {}
                    Page::Prompt => {
                        prompt.input(key.code, key.modifiers);
                    }
                    Page::Editor => {
                        editor.input(key.code, key.modifiers);
                    }
                },
            },
            _ => {}
        }
    }

    terminal.leave_tui()
}
