//! Police bitmap de `text.bmp` : glyphes de 5x6 sur une grille de 31 colonnes × 3 lignes.

/// Largeur et hauteur d'un glyphe.
pub const CHAR_W: i32 = 5;
pub const CHAR_H: i32 = 6;

/// Position (colonne, ligne) du glyphe d'espace.
pub const SPACE: (i32, i32) = (30, 0);

/// Position (colonne, ligne) du glyphe d'un caractère dans `text.bmp`.
///
/// Minuscules → majuscules ; accents ramenés à la lettre de base ; tout caractère inconnu
/// s'affiche comme une espace (comportement de Winamp).
pub fn glyph(c: char) -> (i32, i32) {
    let c = fold(c);
    match c {
        'A'..='Z' => (c as i32 - 'A' as i32, 0),
        '"' => (26, 0),
        '@' => (27, 0),
        ' ' => SPACE,
        '0'..='9' => (c as i32 - '0' as i32, 1),
        '…' => (10, 1),
        '.' => (11, 1),
        ':' => (12, 1),
        '(' => (13, 1),
        ')' => (14, 1),
        '-' => (15, 1),
        '\'' => (16, 1),
        '!' => (17, 1),
        '_' => (18, 1),
        '+' => (19, 1),
        '\\' => (20, 1),
        '/' => (21, 1),
        '[' | '{' | '<' => (22, 1),
        ']' | '}' | '>' => (23, 1),
        '^' | '~' => (24, 1),
        '&' => (25, 1),
        '%' => (26, 1),
        ',' => (27, 1),
        '=' => (28, 1),
        '$' => (29, 1),
        '#' => (30, 1),
        'Å' => (0, 2),
        'Ö' => (1, 2),
        'Ä' => (2, 2),
        '?' => (3, 2),
        '*' => (4, 2),
        _ => SPACE,
    }
}

/// Ramène un caractère vers ce que la police sait dessiner.
fn fold(c: char) -> char {
    let up = match c {
        'å' => 'Å',
        'ö' => 'Ö',
        'ä' => 'Ä',
        '`' | '´' | '‘' | '’' => '\'',
        '“' | '”' | '«' | '»' => '"',
        '–' | '—' => '-',
        '°' => '0',
        'ƒ' => 'F',
        _ => c,
    };
    if up.is_ascii() {
        return up.to_ascii_uppercase();
    }
    match up {
        'Å' | 'Ö' | 'Ä' | '…' => up,
        'à' | 'á' | 'â' | 'ã' | 'À' | 'Á' | 'Â' | 'Ã' | 'æ' | 'Æ' => 'A',
        'ç' | 'Ç' => 'C',
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'E',
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
        'ñ' | 'Ñ' => 'N',
        'ò' | 'ó' | 'ô' | 'õ' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ø' | 'œ' | 'Œ' => 'O',
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
        'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'Y',
        'ß' => 'S',
        'ð' | 'Ð' => 'D',
        'þ' | 'Þ' => 'P',
        _ => up,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_digits() {
        assert_eq!(glyph('a'), (0, 0));
        assert_eq!(glyph('Z'), (25, 0));
        assert_eq!(glyph('7'), (7, 1));
        assert_eq!(glyph('é'), glyph('E'));
        assert_eq!(glyph('ñ'), glyph('N'));
        assert_eq!(glyph('ä'), (2, 2));
        assert_eq!(glyph('<'), glyph('['));
    }

    #[test]
    fn unknown_is_space() {
        assert_eq!(glyph('漢'), SPACE);
        assert_eq!(glyph('\u{1F600}'), SPACE);
    }
}
