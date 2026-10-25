use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const STYLES: &[(&str, &str)] = &[
    ("fass", "Sharp, Solid"),
    ("fasr", "Sharp, Regular"),
    ("fasl", "Sharp, Light"),
    ("fast", "Sharp, Thin"),
    ("fad", "Duotone, Solid"),
    ("fadr", "Duotone, Regular"),
    ("fadl", "Duotone, Light"),
    ("fadt", "Duotone, Thin"),
    ("fas", "Classic, Solid"),
    ("far", "Classic, Regular"),
    ("fal", "Classic, Light"),
    ("fat", "Classic, Thin"),
    ("fab", "Classic, Brands"),
    ("fasds", "Sharp Duotone, Solid"),
    ("fasdr", "Sharp Duotone, Regular"),
    ("fasdl", "Sharp Duotone, Light"),
    ("fasdt", "Sharp Duotone, Thin"),
];
pub const COPIES: &[(&str, &str)] = &[
    ("copyIconName", "Copy Icon Name"),
    ("copyIconClasses", "Copy Icon Classes"),
    ("copyAsSvg", "Copy as SVG"),
    ("copyIconGlyph", "Copy Icon Glyph"),
    ("copyIconUnicode", "Copy Icon Unicode"),
];
pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

// Intentionally not Debug: preferences contain a credential.
#[derive(Clone)]
pub struct Preferences {
    pub token: String,
    pub pro: bool,
    pub style: String,
    pub primary: String,
    pub remember: bool,
    pub filter: String,
}
impl Preferences {
    pub fn parse(value: &Value) -> Result<Self, String> {
        let string = |key: &str, default: &str| -> Result<String, String> {
            match value.get(key) {
                None | Some(Value::Null) => Ok(default.into()),
                Some(Value::String(s)) if s.len() <= 4096 => Ok(s.clone()),
                _ => Err("Invalid preference".into()),
            }
        };
        let token = string("API_TOKEN", "")?;
        if !token.is_empty() && (token.len() > 2048 || !token.bytes().all(|b| b.is_ascii_graphic()))
        {
            return Err("Invalid API token configuration".into());
        }
        let pro = !token.is_empty();
        let style = if pro {
            string("STYLE_PREFERENCE", "fas")?
        } else {
            "fas".into()
        };
        if !STYLES.iter().any(|(key, _)| *key == style) {
            return Err("Unknown icon style".into());
        }
        let primary = string("PRIMARY_ACTION", "copyIconName")?;
        let remember = match value.get("REMEMBER_LAST_KIT") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            _ => return Err("Invalid remember preference".into()),
        };
        Ok(Self {
            token: if pro {
                token
            } else {
                crate::api::FREE_TOKEN.into()
            },
            pro,
            style,
            primary,
            remember,
            filter: string("KIT_FILTER", "")?,
        })
    }
    pub fn scope(&self, profile: &str) -> String {
        digest(
            &json!([
                profile,
                self.token,
                self.pro,
                self.style,
                self.primary,
                self.remember,
                self.filter
            ])
            .to_string(),
        )
    }
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct FamilyStyle {
    pub prefix: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct Svg {
    pub html: String,
    #[serde(rename = "familyStyle")]
    pub family_style: FamilyStyle,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct Icon {
    pub id: String,
    pub unicode: String,
    pub svgs: Vec<Svg>,
}
#[derive(Clone, Deserialize)]
pub struct Kit {
    pub name: String,
    pub token: String,
    #[serde(rename = "iconUploads", default)]
    pub uploads: Vec<Upload>,
}
#[derive(Clone, Deserialize)]
pub struct Upload {
    pub name: String,
    #[serde(default)]
    pub unicode: Value,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "pathData")]
    pub paths: Value,
}
impl Kit {
    pub fn opaque_id(&self) -> String {
        format!("kit:{}", digest(&self.token))
    }
}
pub fn filtered_kits(kits: &[Kit], filter: &str) -> Vec<Kit> {
    let terms: Vec<_> = filter
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    kits.iter()
        .filter(|k| {
            terms.is_empty()
                || terms
                    .iter()
                    .any(|s| *s == k.name.to_lowercase() || *s == k.token.to_lowercase())
        })
        .cloned()
        .collect()
}
pub fn search_body(query: &str, prefix: &str) -> Result<Value, String> {
    let name = STYLES
        .iter()
        .find(|(key, _)| *key == prefix)
        .ok_or("Unknown style")?
        .1;
    let (family, style) = name.split_once(", ").unwrap();
    // Equivalent literal encoding avoids relying on an undocumented GraphQL variable scalar.
    let literal = serde_json::to_string(query).map_err(|_| "Invalid query")?;
    Ok(
        json!({"query":format!("query Search {{ search(query:{literal}, version: \"6.7.2\", first: 48) {{ id unicode svgs(filter: {{ familyStyles: [ {{ family: {}, style: {} }} {{ family: CLASSIC, style: BRANDS }} ] }}) {{ html familyStyle {{ prefix }} }} }} }}",family.replace(' ',"_").to_uppercase(),style.to_uppercase())}),
    )
}
pub fn kits_body(uploads: bool) -> Value {
    json!({"query": if uploads { "query KitIcons { me { kits { name token iconUploads { name unicode width height pathData } } } }" } else { "query Kits { me { kits { name token } } }" }})
}
pub fn kit_icons(kits: &[Kit], selected: &str, query: &str) -> Result<Vec<Icon>, String> {
    // Source's fallback to the first kit is retained; selector tokens never enter views.
    let kit = kits
        .iter()
        .find(|k| k.opaque_id() == selected)
        .or_else(|| kits.first());
    let mut icons = Vec::new();
    let q = query.trim().to_lowercase();
    if let Some(kit) = kit {
        for upload in &kit.uploads {
            let unicode = match &upload.unicode {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Null => String::new(),
                _ => return Err("Invalid kit unicode".into()),
            };
            if !q.is_empty()
                && !upload.name.to_lowercase().contains(&q)
                && !unicode.to_lowercase().contains(&q)
            {
                continue;
            }
            if upload.width == 0
                || upload.height == 0
                || upload.width > 16384
                || upload.height > 16384
            {
                return Err("Invalid kit dimensions".into());
            }
            let paths = match &upload.paths {
                Value::String(s) => vec![s.as_str()],
                Value::Array(a) if a.len() <= 64 => a
                    .iter()
                    .map(|x| x.as_str().ok_or("Invalid path"))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("Invalid paths".into()),
            };
            if paths.iter().any(|d| !valid_path(d)) {
                return Err("Unsafe kit path".into());
            }
            let html = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\">{}</svg>",
                upload.width,
                upload.height,
                paths.iter().fold(String::new(), |mut out, d| {
                    out.push_str("<path d=\"");
                    out.push_str(d);
                    out.push_str("\"/>");
                    out
                })
            );
            icons.push(Icon {
                id: upload.name.clone(),
                unicode,
                svgs: vec![Svg {
                    html,
                    family_style: FamilyStyle {
                        prefix: "kit".into(),
                    },
                }],
            });
        }
    }
    if icons.len() > 512 {
        return Err("Too many kit icons".into());
    }
    validate_icons(&icons)?;
    Ok(icons)
}
pub fn validate_icons(icons: &[Icon]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for icon in icons {
        if icon.id.is_empty()
            || icon.id.len() > 256
            || icon.id.chars().any(char::is_control)
            || !ids.insert(&icon.id)
            || icon.unicode.len() > 32
            || icon.svgs.len() > 32
        {
            return Err("Invalid icon metadata".into());
        }
        for svg in &icon.svgs {
            if svg.html.len() > 65536
                || (svg.family_style.prefix != "kit"
                    && !STYLES.iter().any(|(p, _)| *p == svg.family_style.prefix))
            {
                return Err("Invalid icon SVG metadata".into());
            }
        }
    }
    Ok(())
}
// String.fromCharCode(parseInt(hex,16)) preserves the original 16-bit wrap.
// JSON cannot transport an unpaired UTF-16 surrogate as a Rust String; use U+FFFD there.
pub fn glyph(hex: &str) -> String {
    let mut text = hex.trim_start();
    let negative = text.starts_with('-');
    if text.starts_with(['-', '+']) {
        text = &text[1..];
    }
    if text.starts_with("0x") || text.starts_with("0X") {
        text = &text[2..];
    }
    let digits: String = text.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    let number = u128::from_str_radix(&digits, 16)
        .map(|n| n as f64)
        .unwrap_or(0.0);
    let number = if negative { -number } else { number };
    let number = number.rem_euclid(65536.0) as u16;
    char::from_u32(u32::from(number))
        .unwrap_or('\u{fffd}')
        .to_string()
}
pub fn copy_text(icon: &Icon, action: &str) -> Result<String, String> {
    let svg = icon.svgs.first().ok_or("Icon has no SVG")?;
    Ok(match action {
        "copyIconName" => icon.id.clone(),
        "copyIconClasses" => {
            let style = if svg.family_style.prefix == "kit" {
                "kit"
            } else {
                STYLES
                    .iter()
                    .find(|(p, _)| *p == svg.family_style.prefix)
                    .ok_or("Unknown prefix")?
                    .1
                    .split_once(", ")
                    .unwrap()
                    .1
            };
            format!("fa-{} fa-{}", style.to_lowercase(), icon.id)
        }
        "copyAsSvg" => svg.html.replace("<path", "<path fill=\"currentColor\""),
        "copyIconGlyph" => glyph(&icon.unicode),
        "copyIconUnicode" => icon.unicode.clone(),
        _ => return Err("Unknown copy action".into()),
    })
}
fn valid_path(d: &str) -> bool {
    !d.is_empty()
        && d.len() <= 60000
        && d.bytes()
            .all(|b| b.is_ascii_digit() || b"MmZzLlHhVvCcSsQqTtAaEe+-. ,\t\r\n".contains(&b))
}
pub fn safe_image(svg: &str) -> Option<String> {
    if svg.len() > 65536 {
        return None;
    }
    let doc = roxmltree::Document::parse_with_options(
        svg,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 256,
        },
    )
    .ok()?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return None;
    }
    // Rebuild a tiny inert subset. Never forward href, CSS, URLs, handlers or arbitrary markup.
    let view = root.attribute("viewBox")?;
    let nums: Vec<_> = view
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<_, _>>()
        .ok()?;
    if nums.len() != 4
        || nums.iter().any(|n| !n.is_finite() || n.abs() > 16384.)
        || nums[2] <= 0.
        || nums[3] <= 0.
    {
        return None;
    }
    let mut paths = String::new();
    for n in root.descendants().filter(|n| n.is_element()).skip(1) {
        if n.tag_name().name() != "path" {
            return None;
        }
        let d = n.attribute("d")?;
        if !valid_path(d) {
            return None;
        }
        let opacity = match n.attribute("opacity") {
            Some(s) => {
                let x = s.parse::<f64>().ok()?;
                if !(0.0..=1.0).contains(&x) {
                    return None;
                }
                format!(" opacity=\"{x}\"")
            }
            None => String::new(),
        };
        paths.push_str(&format!("<path fill=\"#888888\"{opacity} d=\"{d}\"/>"));
    }
    if paths.is_empty() {
        return None;
    }
    Some(format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{view}\">{paths}</svg>"
        ))
    ))
}
