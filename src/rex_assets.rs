use rltk::rex::XpFile;

rltk::embedded_resource!(MENU_BG, "../resources/menu_bg.xp");

pub struct RexAssets {
    pub menu: XpFile,
}

impl RexAssets {
    #[allow(clippy::new_without_default)]
    pub fn new() -> RexAssets {
        rltk::link_resource!(MENU_BG, "../resources/menu_bg.xp");

        RexAssets {
            menu: XpFile::from_resource("../resources/menu_bg.xp").unwrap(),
        }
    }
}
