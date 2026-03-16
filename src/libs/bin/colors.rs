use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

use common_macros::hash_map;

use crate::{Expression, RuntimeError, libs::BuiltinInfo, reg_info};

// pub fn regist_color_lazy() -> LazyModule {
//     reg_lazy!({ all })
// }

// pub fn regist_color_info() -> BTreeMap<&'static str, BuiltinInfo> {
//     reg_info!({
//         all => "list all color_name for True Color", "[skip_colorized?]"
//     })
// }

pub fn regist_const_color() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        BLACK => "", ""
        RED => "", ""
        GREEN => "", ""
        YELLOW => "", ""
        BLUE => "", ""
        MAGENTA => "", ""
        CYAN => "", ""
        GRAY => "", ""

        LIGHT_BLACK => "", ""
        LIGHT_RED => "", ""
        LIGHT_GREEN => "", ""
        LIGHT_YELLOW => "", ""
        LIGHT_BLUE => "", ""
        LIGHT_MAGENTA => "", ""
        LIGHT_CYAN => "", ""
        LIGHT_GRAY => "", ""

        BG_BLACK => "", ""
        BG_RED => "", ""
        BG_GREEN => "", ""
        BG_YELLOW => "", ""
        BG_BLUE => "", ""
        BG_MAGENTA => "", ""
        BG_CYAN => "", ""
        BG_GRAY => "", ""

        BG_LIGHT_BLACK => "", ""
        BG_LIGHT_RED => "", ""
        BG_LIGHT_GREEN => "", ""
        BG_LIGHT_YELLOW => "", ""
        BG_LIGHT_BLUE => "", ""
        BG_LIGHT_MAGENTA => "", ""
        BG_LIGHT_CYAN => "", ""
        BG_LIGHT_GRAY => "", ""

        RESET => "", ""

        aliceblue => "",""
        antiquewhite => "",""
        aqua => "",""
        aquamarine => "",""
        azure => "",""
        beige => "",""
        bisque => "",""
        black => "",""
        blanchedalmond => "",""
        blue => "",""
        blueviolet => "",""
        brown => "",""
        burlywood => "",""
        cadetblue => "",""
        chartreuse => "",""
        chocolate => "",""
        coral => "",""
        cornflowerblue => "",""
        cornsilk => "",""
        crimson => "",""
        cyan => "",""
        darkblue => "",""
        darkcyan => "",""
        darkgoldenrod => "",""
        darkgray => "",""
        darkgreen => "",""
        darkgrey => "",""
        darkkhaki => "",""
        darkmagenta => "",""
        darkolivegreen => "",""
        darkorange => "",""
        darkorchid => "",""
        darkred => "",""
        darksalmon => "",""
        darkseagreen => "",""
        darkslateblue => "",""
        darkslategrey => "",""
        darkturquoise => "",""
        darkviolet => "",""
        deeppink => "",""
        deepskyblue => "",""
        dimgray => "",""
        dodgerblue => "",""
        firebrick => "",""
        floralwhite => "",""
        forestgreen => "",""
        fuchsia => "",""
        gainsboro => "",""
        ghostwhite => "",""
        gold => "",""
        goldenrod => "",""
        gray => "",""
        green => "",""
        greenyellow => "",""
        honeydew => "",""
        hotpink => "",""
        indianred => "",""
        indigo => "",""
        ivory => "",""
        khaki => "",""
        lavender => "",""
        lavenderblush => "",""
        lawngreen => "",""
        lemonchiffon => "",""
        lightblue => "",""
        lightcoral => "",""
        lightcyan => "",""
        lightgoldenrodyellow => "",""
        lightgray => "",""
        lightgreen => "",""
        lightgrey => "",""
        lightpink => "",""
        lightsalmon => "",""
        lightseagreen => "",""
        lightskyblue => "",""
        lightslategray => "",""
        lightsteelblue => "",""
        lightyellow => "",""
        lime => "",""
        limegreen => "",""
        linen => "",""
        magenta => "",""
        maroon => "",""
        mediumaquamarine => "",""
        mediumblue => "",""
        mediumorchid => "",""
        mediumpurple => "",""
        mediumseagreen => "",""
        mediumslateblue => "",""
        mediumspringgreen => "",""
        mediumturquoise => "",""
        mediumvioletred => "",""
        midnightblue => "",""
        mintcream => "",""
        mistyrose => "",""
        moccasin => "",""
        navajowhite => "",""
        navy => "",""
        oldlace => "",""
        olive => "",""
        olivedrab => "",""
        orange => "",""
        orangered => "",""
        orchid => "",""
        palegoldenrod => "",""
        palegreen => "",""
        paleturquoise => "",""
        palevioletred => "",""
        papayawhip => "",""
        peachpuff => "",""
        peru => "",""
        pink => "",""
        plum => "",""
        powderblue => "",""
        purple => "",""
        rebeccapurple => "",""
        red => "",""
        rosybrown => "",""
        royalblue => "",""
        saddlebrown => "",""
        salmon => "",""
        sandybrown => "",""
        seagreen => "",""
        seashell => "",""
        sienna => "",""
        silver => "",""
        skyblue => "",""
        slateblue => "",""
        slategray => "",""
        snow => "",""
        springgreen => "",""
        steelblue => "",""
        tan => "",""
        teal => "",""
        thistle => "",""
        tomato => "",""
        turquoise => "",""
        violet => "",""
        wheat => "",""
        white => "",""
        whitesmoke => "",""
        yellow => "",""
        yellowgreen => "",""

        BG_aliceblue => "",""
        BG_antiquewhite => "",""
        BG_aqua => "",""
        BG_aquamarine => "",""
        BG_azure => "",""
        BG_beige => "",""
        BG_bisque => "",""
        BG_black => "",""
        BG_blanchedalmond => "",""
        BG_blue => "",""
        BG_blueviolet => "",""
        BG_brown => "",""
        BG_burlywood => "",""
        BG_cadetblue => "",""
        BG_chartreuse => "",""
        BG_chocolate => "",""
        BG_coral => "",""
        BG_cornflowerblue => "",""
        BG_cornsilk => "",""
        BG_crimson => "",""
        BG_cyan => "",""
        BG_darkblue => "",""
        BG_darkcyan => "",""
        BG_darkgoldenrod => "",""
        BG_darkgray => "",""
        BG_darkgreen => "",""
        BG_darkgrey => "",""
        BG_darkkhaki => "",""
        BG_darkmagenta => "",""
        BG_darkolivegreen => "",""
        BG_darkorange => "",""
        BG_darkorchid => "",""
        BG_darkred => "",""
        BG_darksalmon => "",""
        BG_darkseagreen => "",""
        BG_darkslateblue => "",""
        BG_darkslategrey => "",""
        BG_darkturquoise => "",""
        BG_darkviolet => "",""
        BG_deeppink => "",""
        BG_deepskyblue => "",""
        BG_dimgray => "",""
        BG_dodgerblue => "",""
        BG_firebrick => "",""
        BG_floralwhite => "",""
        BG_forestgreen => "",""
        BG_fuchsia => "",""
        BG_gainsboro => "",""
        BG_ghostwhite => "",""
        BG_gold => "",""
        BG_goldenrod => "",""
        BG_gray => "",""
        BG_green => "",""
        BG_greenyellow => "",""
        BG_honeydew => "",""
        BG_hotpink => "",""
        BG_indianred => "",""
        BG_indigo => "",""
        BG_ivory => "",""
        BG_khaki => "",""
        BG_lavender => "",""
        BG_lavenderblush => "",""
        BG_lawngreen => "",""
        BG_lemonchiffon => "",""
        BG_lightblue => "",""
        BG_lightcoral => "",""
        BG_lightcyan => "",""
        BG_lightgoldenrodyellow => "",""
        BG_lightgray => "",""
        BG_lightgreen => "",""
        BG_lightgrey => "",""
        BG_lightpink => "",""
        BG_lightsalmon => "",""
        BG_lightseagreen => "",""
        BG_lightskyblue => "",""
        BG_lightslategray => "",""
        BG_lightsteelblue => "",""
        BG_lightyellow => "",""
        BG_lime => "",""
        BG_limegreen => "",""
        BG_linen => "",""
        BG_magenta => "",""
        BG_maroon => "",""
        BG_mediumaquamarine => "",""
        BG_mediumblue => "",""
        BG_mediumorchid => "",""
        BG_mediumpurple => "",""
        BG_mediumseagreen => "",""
        BG_mediumslateblue => "",""
        BG_mediumspringgreen => "",""
        BG_mediumturquoise => "",""
        BG_mediumvioletred => "",""
        BG_midnightblue => "",""
        BG_mintcream => "",""
        BG_mistyrose => "",""
        BG_moccasin => "",""
        BG_navajowhite => "",""
        BG_navy => "",""
        BG_oldlace => "",""
        BG_olive => "",""
        BG_olivedrab => "",""
        BG_orange => "",""
        BG_orangered => "",""
        BG_orchid => "",""
        BG_palegoldenrod => "",""
        BG_palegreen => "",""
        BG_paleturquoise => "",""
        BG_palevioletred => "",""
        BG_papayawhip => "",""
        BG_peachpuff => "",""
        BG_peru => "",""
        BG_pink => "",""
        BG_plum => "",""
        BG_powderblue => "",""
        BG_purple => "",""
        BG_rebeccapurple => "",""
        BG_red => "",""
        BG_rosybrown => "",""
        BG_royalblue => "",""
        BG_saddlebrown => "",""
        BG_salmon => "",""
        BG_sandybrown => "",""
        BG_seagreen => "",""
        BG_seashell => "",""
        BG_sienna => "",""
        BG_silver => "",""
        BG_skyblue => "",""
        BG_slateblue => "",""
        BG_slategray => "",""
        BG_snow => "",""
        BG_springgreen => "",""
        BG_steelblue => "",""
        BG_tan => "",""
        BG_teal => "",""
        BG_thistle => "",""
        BG_tomato => "",""
        BG_turquoise => "",""
        BG_violet => "",""
        BG_wheat => "",""
        BG_white => "",""
        BG_whitesmoke => "",""
        BG_yellow => "",""
        BG_yellowgreen => "",""

        FGX_ffffff => "true color for front ground in hex format","<000000..ffffff>"
        BGX_ffffff => "true color for back ground in hex format","<000000..ffffff>"
        FG_256 => "256 color for front ground","<1..=256>"
        BG_256 => "256 color for back ground","<1..=256>"
    })
}

pub fn regist_const_style() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        NORMAL => "",""
        BOLD => "",""
        DIM => "",""
        ITALIC => "",""
        UNDERLINE => "",""
        BLINK => "",""
        REVERSE => "",""
        HIDDEN => "",""
        STRIKE => "",""

        RESET => "",""
        RESET_NORMAL => "",""
        RESET_BOLD => "",""
        RESET_DIM => "",""
        RESET_ITALIC => "",""
        RESET_UNDERLINE => "",""
        RESET_BLINK => "",""
        RESET_REVERSE => "",""
        RESET_HIDDEN => "",""
        RESET_STRIKE => "",""
    })
}

pub static COLOR_MAP: LazyLock<HashMap<&'static str, (i64, i64, i64)>> =
    LazyLock::new(|| init_color_map());

fn init_color_map() -> HashMap<&'static str, (i64, i64, i64)> {
    hash_map! {
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "black" => (0, 0, 0),
        "blanchedalmond" => (255, 235, 205),
        "blue" => (0, 0, 255),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "cyan" => (0, 255, 255),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkgrey" => (169, 169, 169),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "fuchsia" => (255, 0, 255),
        "gainsboro" => (221, 221, 221),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "gray" => (128, 128, 128),
        "green" => (0, 255, 0),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 245, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightgrey" => (211, 211, 211),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "magenta" => (255, 0, 255),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 209, 209),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (183, 105, 224),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 150),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orange" => (255, 165, 0),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (238, 130, 238),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "purple" => (128, 0, 128),
        "rebeccapurple" => (102, 51, 153),
        "red" => (255, 0, 0),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 128),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "white" => (255, 255, 255),
        "whitesmoke" => (245, 245, 245),
        "yellow" => (255, 255, 0),
        "yellowgreen" => (154, 255, 50),
    }
}

pub fn handle_color(arg: &str, ctx: &Expression) -> Result<Expression, RuntimeError> {
    let s = if arg == "RESET" {
        Ok("\x1b[0m".to_string())
    } else if let Some(bg_color) = arg.strip_prefix("BG_") {
        match bg_color.len() {
            ..=3 => get_256_bgcolor(bg_color, ctx),
            _ => {
                if arg.chars().next().is_some_and(|x| x.is_uppercase()) {
                    handle_base_color(bg_color, true, ctx)
                } else {
                    true_color_by_name(bg_color, true, ctx)
                }
            }
        }
    } else if let Some(bg_color) = arg.strip_prefix("BGX_") {
        true_color_by_hex(bg_color, true, ctx)
    } else if let Some(fg_color) = arg.strip_prefix("FG_") {
        match fg_color.len() {
            ..=3 => get_256_color(fg_color, ctx),
            _ => {
                if arg.chars().next().is_some_and(|x| x.is_uppercase()) {
                    handle_base_color(fg_color, false, ctx)
                } else {
                    true_color_by_name(fg_color, false, ctx)
                }
            }
        }
    } else if let Some(fg_color) = arg.strip_prefix("FGX_") {
        true_color_by_hex(fg_color, false, ctx)
    } else {
        if arg.chars().next().is_some_and(|x| x.is_uppercase()) {
            handle_base_color(arg, false, ctx)
        } else {
            true_color_by_name(arg, false, ctx)
        }
    };

    // let s = if let Some(hex) = arg.strip_prefix("t_") {
    //     true_color_by_hex(hex, false, ctx)
    // } else if let Some(x) = arg.strip_prefix("x_") {
    //     get_256_color(x, ctx)
    // } else if arg.chars().all(|x| x.is_uppercase()) {
    //     get_base_color(arg, ctx)
    // } else {
    //     true_color_by_name(arg, false, ctx)
    // };
    s.map(Expression::String)
}

// pub fn handle_color_bg(arg: &str, ctx: &Expression) -> Result<Expression, RuntimeError> {
//     let s = if let Some(hex) = arg.strip_prefix("t_") {
//         true_color_by_hex(hex, true, ctx)
//     } else if let Some(x) = arg.strip_prefix("x_") {
//         get_256_bgcolor(x, ctx)
//     } else if arg.chars().all(|x| x.is_uppercase()) {
//         get_base_bgcolor(arg, ctx)
//     } else {
//         true_color_by_name(arg, true, ctx)
//     };
//     s.map(Expression::String)
// }

fn get_256_color(arg: &str, ctx: &Expression) -> Result<String, RuntimeError> {
    match arg.parse::<usize>() {
        Ok(c) if c < 256 && c > 0 => Ok(format!("\x1b[38;5;{}m", c)),
        _ => Err(RuntimeError::common(
            "invalid 256 color".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn get_256_bgcolor(arg: &str, ctx: &Expression) -> Result<String, RuntimeError> {
    match arg.parse::<usize>() {
        Ok(c) if c <= 256 && c > 0 => Ok(format!("\x1b[48;5;{}m", c)),
        _ => Err(RuntimeError::common(
            "invalid 256 color".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn true_color_by_name(
    color_spec: &str,
    is_bg: bool,
    ctx: &Expression,
) -> Result<String, RuntimeError> {
    let color_code = if let Some((r, g, b)) = COLOR_MAP.get(color_spec) {
        format!("{};{};{}", r, g, b)
    } else {
        return Err(RuntimeError::common(
            "invalid true color name".into(),
            ctx.clone(),
            0,
        ));
    };

    let prefix = if is_bg { "48" } else { "38" };
    Ok(format!("\x1b[{};2;{}m", prefix, color_code))
}

pub fn true_color_by_hex(hex: &str, is_bg: bool, ctx: &Expression) -> Result<String, RuntimeError> {
    // Parse hex color
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        let color_code = format!("{};{};{}", r, g, b);

        let prefix = if is_bg { "48" } else { "38" };
        Ok(format!("\x1b[{};2;{}m", prefix, color_code))
    } else {
        return Err(RuntimeError::common(
            "invalid hex color format".into(),
            ctx.clone(),
            0,
        ));
    }
}

/// style
pub fn handle_style(name: &str, ctx: &Expression) -> Result<Expression, RuntimeError> {
    let code = if name == "RESET" {
        0 as usize
    } else if let Some(reset) = name.strip_prefix("RESET_") {
        get_style_code(reset, ctx)? + 20
    } else {
        get_style_code(name, ctx)?
    };

    Ok(Expression::String(format!("\x1b[{}m", code)))
}

pub fn get_style_code(name: &str, ctx: &Expression) -> Result<usize, RuntimeError> {
    Ok(match name {
        "NORMAL" => 0,
        "BOLD" => 1,
        "DIM" => 2,
        "ITALIC" => 3,
        "UNDERLINE" => 4,
        "BLINK" => 5,
        "REVERSE" => 7,
        "HIDDEN" => 8,
        "STRIKE" => 9,
        _ => {
            return Err(RuntimeError::common(
                "invalid style name".into(),
                ctx.clone(),
                0,
            ));
        }
    })
}

fn handle_base_color(name: &str, is_bg: bool, ctx: &Expression) -> Result<String, RuntimeError> {
    let code = match is_bg {
        false => {
            if let Some(light) = name.strip_prefix("LIGHT_") {
                get_base_color_code(light, ctx)? + 60
            } else {
                get_base_color_code(name, ctx)?
            }
        }
        true => {
            if let Some(light) = name.strip_prefix("LIGHT_") {
                get_base_color_code(light, ctx)? + 70
            } else {
                get_base_color_code(name, ctx)? + 10
            }
        }
    };
    Ok(format!("\x1b[{}m", code))
}

fn get_base_color_code(name: &str, ctx: &Expression) -> Result<usize, RuntimeError> {
    Ok(match name {
        "BLACK" => 30,
        "RED" => 31,
        "GREEN" => 32,
        "BROWN" => 33,
        "BLUE" => 34,
        "PURPLe" => 35,
        "CYAN" => 36,
        "GRAY" => 37,
        _ => {
            return Err(RuntimeError::common(
                "invalid hex color format".into(),
                ctx.clone(),
                0,
            ));
        }
    })
}
