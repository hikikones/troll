use terminal::*;
use widgets2::{Block, KittyDeleteAll, Markup};

pub struct App {
    frame: Framebuffer,
    markup: Markup,
    index: usize,
    is_running: bool,
}

impl App {
    pub fn new(args: crate::cli::Args) -> Self {
        Self {
            frame: Framebuffer::new(),
            markup: Markup::new(args.assets.unwrap()) // TODO
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            index: 0,
            is_running: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        self.frame.query_size()?;
        self.render(terminal)?;

        while self.is_running {
            let Some(event) = Terminal::read()? else {
                continue;
            };

            let action = self.handle_event(event, terminal)?;
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: TerminalEvent,
        terminal: &mut Terminal,
    ) -> std::io::Result<Action> {
        let action = match event {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Esc => Action::Quit,
                KeyCode::Left => {
                    if self.index == 0 {
                        self.index = 3
                    } else {
                        self.index -= 1;
                    }
                    Action::Render
                }
                KeyCode::Right => {
                    if self.index == 3 {
                        self.index = 0
                    } else {
                        self.index += 1;
                    }
                    Action::Render
                }
                KeyCode::Char('e') => {
                    terminal.temp_leave(|| Ok(()))?;
                    Action::Clear
                }
                _ => {
                    if self.markup.input(key.code) {
                        Action::Render
                    } else {
                        Action::None
                    }
                }
            },
            TerminalEvent::Resize => {
                self.frame.query_size()?;
                Action::Render
            }
        };

        Ok(action)
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> std::io::Result<()> {
        match action {
            Action::None => {}
            Action::Render => {
                self.render(terminal)?;
            }
            Action::Clear => {
                self.frame.increment_generation();
                self.render(terminal)?;
            }
            Action::Quit => {
                self.is_running = false;
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        let mut frame = std::mem::take(&mut self.frame);

        frame.print_fmt(KittyDeleteAll);

        let area = frame.area().inner(Margin::proportional(3));
        Block::rectangle().render(area, &mut frame);
        let inner = area.inner(Margin::all(1));

        let text = match self.index {
            0 => MARKUP,
            1 => LOREM,
            2 => EMOJI_TAB,
            3 => PICARD,
            _ => unreachable!(),
        };

        self.markup.render(inner, &mut frame, text);

        frame.flush(&mut terminal.lock())?;
        self.frame = frame;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    None,
    Render,
    Clear,
    Quit,
}

#[derive(Debug)]
pub struct AppError {
    message: String,
    source: Option<AppErrorSource>,
}

impl AppError {
    pub const fn new(message: String, source: Option<AppErrorSource>) -> Self {
        Self { message, source }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| match source {
            AppErrorSource::Io(err) => err as _,
            AppErrorSource::Config(err) => err as _,
        })
    }
}

#[derive(Debug)]
pub enum AppErrorSource {
    Io(std::io::Error),
    Config(crate::settings::ConfigError),
}

const LOREM: &str = r#"
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Donec fermentum ipsum nec sagittis feugiat. Curabitur pulvinar et orci luctus faucibus. In erat justo, placerat et risus quis, cursus elementum mi. Donec non leo est. Etiam consectetur, lectus nec auctor sodales, velit arcu tincidunt justo, mattis dictum odio libero ac enim. Morbi maximus, tortor id ornare tristique, purus mi pulvinar urna, vel accumsan ante nunc vel urna. Maecenas ligula elit, tempor eget augue ut, auctor blandit ligula. Mauris maximus condimentum aliquam. Nam purus enim, ornare ut suscipit et, bibendum a ex. Donec euismod velit quis nisi convallis, rhoncus lacinia sapien aliquam. Ut dolor magna, imperdiet eu arcu vitae, consectetur feugiat velit. Donec eu dolor eu tortor rutrum egestas quis in orci. Maecenas pulvinar, massa eget fringilla convallis, dolor tellus luctus elit, in tempus nibh dolor at dolor. Nulla viverra et justo sit amet rhoncus. Fusce porttitor odio in lectus ullamcorper vehicula.

Nullam pharetra augue non leo maximus convallis a sit amet risus. Integer congue laoreet efficitur. Interdum et malesuada fames ac ante ipsum primis in faucibus. Quisque vestibulum, nisl vitae porttitor maximus, nulla mauris cursus ligula, vel consectetur magna dolor ac nulla. Integer at pharetra nisi, quis congue ante. Nam et varius purus. Maecenas placerat sapien at ante cursus, at volutpat felis facilisis. Fusce id metus vel urna molestie tincidunt. Phasellus sed dapibus ligula.

Vivamus nec dui pellentesque, interdum dui vitae, egestas ex. Morbi malesuada porta vehicula. Donec dapibus mattis arcu, vel bibendum diam lobortis et. Curabitur sit amet felis aliquet enim mollis semper. Mauris diam orci, rutrum sed neque vel, malesuada sollicitudin augue. Integer vehicula dolor consectetur tincidunt imperdiet. Donec at dui urna. Duis et turpis in diam ornare volutpat.

Mauris suscipit imperdiet mi et semper. Nam nec lorem sagittis, lobortis ligula convallis, hendrerit tortor. Morbi dapibus magna ut sollicitudin placerat. Etiam sodales varius ante convallis hendrerit. Duis varius quam et tincidunt vestibulum. Fusce facilisis elit eu ligula consequat, dignissim elementum diam aliquet. Maecenas ipsum tellus, condimentum sit amet ultrices et, semper id nibh. Fusce varius porttitor dui vitae fermentum. Nam placerat, nunc eget tempus egestas, enim nisl scelerisque erat, in venenatis leo leo a leo. Nam quis nibh vitae turpis efficitur lobortis id a metus.

In aliquet dui sapien, ut semper elit sodales sed. Proin quis libero luctus libero scelerisque ornare eget id mi. Cras accumsan arcu ut ante pharetra fringilla. Sed feugiat placerat dolor, et feugiat lorem iaculis id. Cras interdum est nec elit molestie, sed aliquam quam hendrerit. Proin sit amet pellentesque enim. Nam bibendum, mauris vel eleifend ultricies, orci sapien hendrerit sem, a tristique lectus nulla a ligula.
"#;

const MARKUP: &str = r#"
=== Heading
# this is a comment
*left* paragraph with *bold*, _italic_ and maybe `verbatim text` that *should wrap* when line becomes _*tooooooooo*_ long..*.*

- item 1
- item 2 with *lots of text* that can wrap to next line but also keeping the indent so it looks nice ohhh yeah
- item 3

; right paragraph

: center paragraph

![ image description text ]( wallpaper.jpg2 )

```rust
fn main() {
    println!("Hello, world!");
}
```

---

=; Heading 2
![  ]( meow.png )

---

=: Heading 3



Lorem ipsum dolor sit amet,
consectetur adipiscing elit.
Donec fermentum ipsum nec sagittis feugiat.
Curabitur pulvinar et orci luctus faucibus.
In erat justo, placerat et risus quis, cursus elementum mi.

$$
\begin{aligned}
\int_{0}^{\infty}
\left(
\sum_{n=1}^{\infty}
\frac{(-1)^{n-1}x^{n}}{n!}
\right)
e^{-x}\,dx
&=
\sum_{n=1}^{\infty}
\frac{(-1)^{n-1}}{n!}
\int_{0}^{\infty}
x^{n}e^{-x}\,dx
\\[1em]
&=
\sum_{n=1}^{\infty}
(-1)^{n-1}
\frac{\Gamma(n+1)}{n!}
\\[1em]
&=
\sum_{n=1}^{\infty}
(-1)^{n-1}
\\[1em]
&=
\frac{1}{2}.
\end{aligned}
$$

---

![ image description text ]( tall2.jpg )

---

another one

---

and another one

![ image description text ]( tall.gif )

= Final Heading
"#;

const EMOJI_TAB: &str = "👻 oijwqwu qwdiowhq  i hio h qiowhqwheqw👻👻 wwq \
qiuwhdidwh👻👻👻❤️\n\nauhui ❤️awudhia\n🧑‍🌾❤️👨‍🦰jfpkw huiw wjwioj ijf \
weoijwioejfiowejfiowjfiowej\n\nthis\tis\ta\tparagraph\twith\ttabs\n\n\
```rust\nfn main() {\n\tprintln!(\"Hello, world!\");\n}\n```";

const PICARD: &str = ":❝_It is possible to commit no mistakes and still lose.\nThat is not a weakness. That is life._❞\n\nWho is this quote from?\n\n---\n\n![Jean-Luc Picard](picard2.jpg)";
