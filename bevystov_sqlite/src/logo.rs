use terminal::{Framebuffer, Rect, Size, TextOptions};

pub enum Logo {
    Tiny,
    Small,
    Medium,
    Large,
}

impl Logo {
    const LOGO_TINY_WIDTH: u16 = 9;
    const LOGO_TINY_HEIGHT: u16 = 1;
    const LOGO_TINY: &str = "TROLLSTOV";

    const LOGO_SMALL_WIDTH: u16 = 28;
    const LOGO_SMALL_HEIGHT: u16 = 5;
    const LOGO_SMALL: &str = r#"
┌┬┐┬─┐┌─┐┬  ┬  ┌─┐┌┬┐┌─┐┬  ┬
 │ ├┬┘│ ││  │  └─┐ │ │ │└┐┌┘
 ┴ ┴└─└─┘┴─┘┴─┘└─┘ ┴ └─┘ └┘ 
"#;

    const LOGO_MEDIUM_WIDTH: u16 = 66;
    const LOGO_MEDIUM_HEIGHT: u16 = 6;
    const LOGO_MEDIUM: &str = r#"
888888 88""Yb  dP"Yb  88     88     .dP"Y8 888888  dP"Yb  Yb    dP
  88   88__dP dP   Yb 88     88     `Ybo."   88   dP   Yb  Yb  dP 
  88   88"Yb  Yb   dP 88  .o 88  .o o.`Y8b   88   Yb   dP   YbdP  
  88   88  Yb  YbodP  88ood8 88ood8 8bodP'   88    YbodP     YP   
"#;

    const LOGO_BIG_WIDTH: u16 = 98;
    const LOGO_BIG_HEIGHT: u16 = 9;
    const LOGO_BIG: &str = r#"
::::::::::: :::::::::   ::::::::  :::        :::        :::::::: ::::::::::: ::::::::  :::     :::
    :+:     :+:    :+: :+:    :+: :+:        :+:       :+:    :+:    :+:    :+:    :+: :+:     :+:
    +:+     +:+    +:+ +:+    +:+ +:+        +:+       +:+           +:+    +:+    +:+ +:+     +:+
    +#+     +#++:++#:  +#+    +:+ +#+        +#+       +#++:++#++    +#+    +#+    +:+ +#+     +:+
    +#+     +#+    +#+ +#+    +#+ +#+        +#+              +#+    +#+    +#+    +#+  +#+   +#+ 
    #+#     #+#    #+# #+#    #+# #+#        #+#       #+#    #+#    #+#    #+#    #+#   #+#+#+#  
    ###     ###    ###  ########  ########## ########## ########     ###     ########      ###    
"#;

    pub const fn from_size(size: Size) -> Self {
        if size.cols > Self::LOGO_BIG_WIDTH + 12 && size.rows > Self::LOGO_BIG_HEIGHT + 3 {
            Self::Large
        } else if size.cols > Self::LOGO_MEDIUM_WIDTH + 8
            && size.rows > Self::LOGO_MEDIUM_HEIGHT + 2
        {
            Self::Medium
        } else if size.cols > Self::LOGO_SMALL_WIDTH + 4 && size.rows > Self::LOGO_SMALL_HEIGHT + 1
        {
            Self::Small
        } else {
            Self::Tiny
        }
    }

    const fn size(&self) -> Size {
        match self {
            Self::Tiny => Size::new(Self::LOGO_TINY_WIDTH, Self::LOGO_TINY_HEIGHT),
            Self::Small => Size::new(Self::LOGO_SMALL_WIDTH, Self::LOGO_SMALL_HEIGHT),
            Self::Medium => Size::new(Self::LOGO_MEDIUM_WIDTH, Self::LOGO_MEDIUM_HEIGHT),
            Self::Large => Size::new(Self::LOGO_BIG_WIDTH, Self::LOGO_BIG_HEIGHT),
        }
    }

    const fn ascii(&self) -> &'static str {
        match self {
            Self::Tiny => Self::LOGO_TINY,
            Self::Small => Self::LOGO_SMALL,
            Self::Medium => Self::LOGO_MEDIUM,
            Self::Large => Self::LOGO_BIG,
        }
    }

    pub fn render(self, area: Rect, frame: &mut Framebuffer) -> Rect {
        let center = area.with_size(self.size()).center(area);

        frame.push_str(self.ascii());
        frame.render(center, TextOptions::paragraph());

        center
    }
}
