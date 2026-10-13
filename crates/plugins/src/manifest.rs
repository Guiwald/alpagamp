//! La description d'un plugin (`alp_info`, JSON) : nom, sorte, auteur, version, description, options.

use std::collections::BTreeMap;

use alpagamp_core::i18n::{Lang, lang};
use serde::Deserialize;

/// Sorte de plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Visualisation : une image par trame.
    Vis,
    /// Effet audio.
    Dsp,
    /// Plugin général (temps, commandes du lecteur).
    General,
}

/// Un texte, éventuellement traduit (`{ "en": …, "fr": … }`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Text {
    Plain(String),
    Translated(BTreeMap<String, String>),
}

impl Default for Text {
    fn default() -> Self {
        Text::Plain(String::new())
    }
}

impl Text {
    /// Le texte dans la langue de l'interface (repli : anglais, puis le premier venu).
    pub fn get(&self) -> &str {
        self.in_lang(lang())
    }

    pub fn in_lang(&self, l: Lang) -> &str {
        match self {
            Text::Plain(s) => s,
            Text::Translated(m) => {
                let wanted: &[&str] = match l {
                    Lang::Va => &["ca-valencia", "va", "ca"],
                    other => &[other.code()],
                };
                wanted
                    .iter()
                    .find_map(|k| m.get(*k))
                    .or_else(|| m.get("en"))
                    .or_else(|| m.values().next())
                    .map(String::as_str)
                    .unwrap_or("")
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            Text::Plain(s) => s.len(),
            Text::Translated(m) => m.iter().map(|(k, v)| k.len() + v.len()).sum(),
        }
    }
}

/// Une valeur possible d'une option.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OptionValue {
    pub value: i32,
    pub label: Text,
}

/// Une option réglable (menu à choix).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OptionDef {
    pub id: String,
    pub label: Text,
    pub values: Vec<OptionValue>,
    #[serde(default)]
    pub default: Option<i32>,
}

impl OptionDef {
    /// Valeur par défaut (sinon la première).
    pub fn default_value(&self) -> i32 {
        self.default.filter(|d| self.values.iter().any(|v| v.value == *d)).or(self.values.first().map(|v| v.value)).unwrap_or(0)
    }
}

/// La description complète.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub name: Text,
    pub kind: Kind,
    #[serde(default)]
    pub author: Text,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: Text,
    #[serde(default)]
    pub options: Vec<OptionDef>,
}

impl Manifest {
    /// Lit et vérifie la description.
    pub fn parse(json: &str) -> Result<Manifest, String> {
        let m: Manifest = serde_json::from_str(json).map_err(|e| format!("description illisible : {e}"))?;
        if m.name.get().trim().is_empty() {
            return Err("plugin sans nom".into());
        }
        if m.name.len() > 2000 || m.version.len() > 64 || m.options.len() > 16 {
            return Err("description trop longue".into());
        }
        for o in &m.options {
            if o.id.is_empty() || o.values.is_empty() || o.values.len() > 32 {
                return Err(format!("option « {} » invalide", o.id));
            }
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_translated() {
        let m = Manifest::parse(
            r#"{"name": {"en": "Bars", "fr": "Barres", "ca": "Barres CA"}, "kind": "vis", "version": "1",
                "options": [{"id": "n", "label": "N", "values": [{"value": 1, "label": "un"}, {"value": 2, "label": "deux"}], "default": 2}]}"#,
        )
        .unwrap();
        assert_eq!(m.kind, Kind::Vis);
        assert_eq!(m.name.in_lang(Lang::Fr), "Barres");
        assert_eq!(m.name.in_lang(Lang::De), "Bars");
        assert_eq!(m.name.in_lang(Lang::Va), "Barres CA");
        assert_eq!(m.options[0].default_value(), 2);
        assert!(Manifest::parse(r#"{"name": "", "kind": "dsp"}"#).is_err());
        assert!(Manifest::parse(r#"{"name": "x", "kind": "autre"}"#).is_err());
    }
}
