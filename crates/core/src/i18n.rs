//! Traductions des menus, préférences et messages (les libellés dessinés dans les skins restent tels quels).
//!
//! Langues : anglais, français, espagnol, allemand, italien, valencien, irlandais (ADR 0014).

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Lang {
    En = 0,
    Fr = 1,
    Es = 2,
    De = 3,
    It = 4,
    /// Valencien (catalan de Valence).
    Va = 5,
    /// Irlandais (gaélique).
    Ga = 6,
}

impl Lang {
    pub const ALL: [Lang; 7] = [Lang::En, Lang::Fr, Lang::Es, Lang::De, Lang::It, Lang::Va, Lang::Ga];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::It => "it",
            Lang::Va => "ca-valencia",
            Lang::Ga => "ga",
        }
    }

    /// Nom de la langue dans la langue elle-même.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Fr => "Français",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::It => "Italiano",
            Lang::Va => "Valencià",
            Lang::Ga => "Gaeilge",
        }
    }

    /// Interprète une étiquette de langue (« fr-FR », « ca-ES-valencia », « ga »…).
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let t = tag.to_ascii_lowercase().replace('_', "-");
        if t.starts_with("ca") {
            // Le catalan (y compris valencien) est servi par la traduction valencienne.
            return Some(Lang::Va);
        }
        let primary = t.split('-').next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.code() == primary || (primary == "va" && *l == Lang::Va))
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(l: Lang) {
    CURRENT.store(l as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    Lang::ALL[CURRENT.load(Ordering::Relaxed) as usize % 7]
}

/// Traduit une clé ; repli sur l'anglais, puis sur la clé elle-même.
pub fn t(key: &str) -> &'static str {
    let l = lang() as usize;
    match TABLE.iter().find(|(k, _)| *k == key) {
        Some((_, tr)) => {
            let s = tr[l];
            if s.is_empty() { tr[0] } else { s }
        }
        None => Box::leak(key.to_string().into_boxed_str()),
    }
}

/// (clé, [en, fr, es, de, it, va, ga])
static TABLE: &[(&str, [&str; 7])] = &[
    ("menu.play_file", ["Play file…", "Lire un fichier…", "Reproducir archivo…", "Datei abspielen…", "Riproduci file…", "Reproduir fitxer…", "Seinn comhad…"]),
    ("menu.play_folder", ["Play folder…", "Lire un dossier…", "Reproducir carpeta…", "Ordner abspielen…", "Riproduci cartella…", "Reproduir carpeta…", "Seinn fillteán…"]),
    ("menu.play_url", ["Play URL…", "Lire une URL…", "Reproducir URL…", "URL abspielen…", "Riproduci URL…", "Reproduir URL…", "Seinn URL…"]),
    ("menu.file_info", ["File info…", "Infos du fichier…", "Información del archivo…", "Dateiinfo…", "Info file…", "Informació del fitxer…", "Eolas comhaid…"]),
    ("menu.playback", ["Playback", "Lecture", "Reproducción", "Wiedergabe", "Riproduzione", "Reproducció", "Athsheinm"]),
    ("menu.previous", ["Previous", "Précédent", "Anterior", "Zurück", "Precedente", "Anterior", "Roimhe seo"]),
    ("menu.play", ["Play", "Lecture", "Reproducir", "Abspielen", "Riproduci", "Reproduir", "Seinn"]),
    ("menu.pause", ["Pause", "Pause", "Pausa", "Pause", "Pausa", "Pausa", "Sos"]),
    ("menu.stop", ["Stop", "Stop", "Detener", "Stopp", "Stop", "Atura", "Stad"]),
    ("menu.next", ["Next", "Suivant", "Siguiente", "Weiter", "Successivo", "Següent", "Ar aghaidh"]),
    ("menu.stop_fadeout", ["Stop with fadeout", "Arrêt en fondu", "Detener con fundido", "Stopp mit Ausblenden", "Stop in dissolvenza", "Atura amb fosa", "Stad le céimniú"]),
    ("menu.stop_after_current", ["Stop after current", "Arrêter après ce morceau", "Detener tras la actual", "Nach aktuellem Titel stoppen", "Ferma dopo il brano", "Atura després de l'actual", "Stad tar éis an rian seo"]),
    ("menu.back5", ["Back 5 seconds", "Reculer de 5 secondes", "Retroceder 5 segundos", "5 Sekunden zurück", "Indietro di 5 secondi", "Arrere 5 segons", "Siar 5 soicind"]),
    ("menu.fwd5", ["Forward 5 seconds", "Avancer de 5 secondes", "Avanzar 5 segundos", "5 Sekunden vor", "Avanti di 5 secondi", "Avant 5 segons", "Ar aghaidh 5 soicind"]),
    ("menu.options", ["Options", "Options", "Opciones", "Optionen", "Opzioni", "Opcions", "Roghanna"]),
    ("menu.always_on_top", ["Always on top", "Toujours au premier plan", "Siempre visible", "Immer im Vordergrund", "Sempre in primo piano", "Sempre al davant", "I gcónaí ar barr"]),
    ("menu.double_size", ["Double size", "Double taille", "Tamaño doble", "Doppelte Größe", "Dimensione doppia", "Mida doble", "Méid dhúbailte"]),
    ("menu.time_remaining", ["Show remaining time", "Afficher le temps restant", "Mostrar tiempo restante", "Restzeit anzeigen", "Mostra tempo rimanente", "Mostra el temps restant", "Taispeáin an t-am atá fágtha"]),
    ("menu.repeat", ["Repeat", "Répéter", "Repetir", "Wiederholen", "Ripeti", "Repetix", "Athdhéan"]),
    ("menu.shuffle", ["Shuffle", "Aléatoire", "Aleatorio", "Zufällig", "Casuale", "Aleatori", "Suaitheadh"]),
    ("menu.manual_advance", ["Manual playlist advance", "Avance manuelle", "Avance manual", "Manueller Titelwechsel", "Avanzamento manuale", "Avanç manual", "Ardú láimhe"]),
    ("menu.skins", ["Skins", "Skins", "Skins", "Skins", "Skin", "Aparences", "Craicne"]),
    ("menu.load_skin", ["Load skin…", "Charger un skin…", "Cargar skin…", "Skin laden…", "Carica skin…", "Carrega una aparença…", "Lódáil craiceann…"]),
    ("menu.default_skin", ["Default skin", "Skin par défaut", "Skin predeterminado", "Standard-Skin", "Skin predefinita", "Aparença per defecte", "Craiceann réamhshocraithe"]),
    ("menu.visualization", ["Visualization", "Visualisation", "Visualización", "Visualisierung", "Visualizzazione", "Visualització", "Léirshamhlú"]),
    ("menu.analyzer", ["Spectrum analyzer", "Analyseur de spectre", "Analizador de espectro", "Spektrumanalysator", "Analizzatore di spettro", "Analitzador d'espectre", "Anailíseoir speictrim"]),
    ("menu.oscilloscope", ["Oscilloscope", "Oscilloscope", "Osciloscopio", "Oszilloskop", "Oscilloscopio", "Oscil·loscopi", "Ascalascóp"]),
    ("menu.vis_off", ["Off", "Éteint", "Apagado", "Aus", "Spento", "Apagat", "As"]),
    ("menu.about", ["About Alpagamp", "À propos d'Alpagamp", "Acerca de Alpagamp", "Über Alpagamp", "Informazioni su Alpagamp", "Quant a Alpagamp", "Maidir le Alpagamp"]),
    ("menu.exit", ["Exit", "Quitter", "Salir", "Beenden", "Esci", "Ix", "Scoir"]),
    ("menu.language", ["Language", "Langue", "Idioma", "Sprache", "Lingua", "Idioma", "Teanga"]),
    ("menu.system_language", ["System language", "Langue du système", "Idioma del sistema", "Systemsprache", "Lingua di sistema", "Idioma del sistema", "Teanga an chórais"]),
    ("dialog.audio_files", ["Audio files", "Fichiers audio", "Archivos de audio", "Audiodateien", "File audio", "Fitxers d'àudio", "Comhaid fuaime"]),
    ("dialog.playlists", ["Playlists", "Playlists", "Listas de reproducción", "Wiedergabelisten", "Playlist", "Llistes de reproducció", "Seinmliostaí"]),
    ("dialog.skins", ["Skins", "Skins", "Skins", "Skins", "Skin", "Aparences", "Craicne"]),
    ("dialog.all_files", ["All files", "Tous les fichiers", "Todos los archivos", "Alle Dateien", "Tutti i file", "Tots els fitxers", "Gach comhad"]),
    ("about.text", [
        "Alpagamp {version}\n\nA music player with classic skins, written in Rust.\nNot affiliated with or endorsed by Winamp.\n\nhttps://github.com/Guiwald/alpagamp",
        "Alpagamp {version}\n\nUn lecteur de musique à skins classiques, écrit en Rust.\nNi affilié à Winamp, ni approuvé par Winamp.\n\nhttps://github.com/Guiwald/alpagamp",
        "Alpagamp {version}\n\nUn reproductor de música con skins clásicos, escrito en Rust.\nSin relación con Winamp ni respaldado por Winamp.\n\nhttps://github.com/Guiwald/alpagamp",
        "Alpagamp {version}\n\nEin Musikplayer mit klassischen Skins, geschrieben in Rust.\nWeder mit Winamp verbunden noch von Winamp unterstützt.\n\nhttps://github.com/Guiwald/alpagamp",
        "Alpagamp {version}\n\nUn lettore musicale con skin classiche, scritto in Rust.\nNon affiliato né approvato da Winamp.\n\nhttps://github.com/Guiwald/alpagamp",
        "Alpagamp {version}\n\nUn reproductor de música amb aparences clàssiques, escrit en Rust.\nSense relació amb Winamp ni avalat per Winamp.\n\nhttps://github.com/Guiwald/alpagamp",
        "Alpagamp {version}\n\nSeinnteoir ceoil le craicne clasaiceacha, scríofa in Rust.\nNíl baint aige le Winamp ná formhuinithe ag Winamp.\n\nhttps://github.com/Guiwald/alpagamp",
    ]),
    ("info.title", ["File info", "Infos du fichier", "Información del archivo", "Dateiinfo", "Info file", "Informació del fitxer", "Eolas comhaid"]),
    ("info.artist", ["Artist", "Artiste", "Artista", "Interpret", "Artista", "Artista", "Ealaíontóir"]),
    ("info.track_title", ["Title", "Titre", "Título", "Titel", "Titolo", "Títol", "Teideal"]),
    ("info.album", ["Album", "Album", "Álbum", "Album", "Album", "Àlbum", "Albam"]),
    ("info.year", ["Year", "Année", "Año", "Jahr", "Anno", "Any", "Bliain"]),
    ("info.genre", ["Genre", "Genre", "Género", "Genre", "Genere", "Gènere", "Seánra"]),
    ("info.length", ["Length", "Durée", "Duración", "Länge", "Durata", "Durada", "Fad"]),
    ("info.format", ["Format", "Format", "Formato", "Format", "Formato", "Format", "Formáid"]),
    ("info.location", ["Location", "Emplacement", "Ubicación", "Ort", "Posizione", "Ubicació", "Suíomh"]),
    ("menu.file", ["File", "Fichier", "Archivo", "Datei", "File", "Fitxer", "Comhad"]),
    ("menu.windows", ["Windows", "Fenêtres", "Ventanas", "Fenster", "Finestre", "Finestres", "Fuinneoga"]),
    ("menu.equalizer", ["Equalizer", "Égaliseur", "Ecualizador", "Equalizer", "Equalizzatore", "Equalitzador", "Cothromóir"]),
    ("menu.playlist_editor", ["Playlist editor", "Éditeur de playlist", "Editor de lista de reproducción", "Wiedergabeliste", "Editor della playlist", "Editor de la llista", "Eagarthóir seinmliosta"]),
    ("menu.jump_to_file", ["Jump to file…", "Aller au morceau…", "Saltar a la pista…", "Zu Titel springen…", "Vai al brano…", "Salta a la pista…", "Léim go rian…"]),
    ("eq.presets", ["Presets", "Préréglages", "Preajustes", "Presets", "Preset", "Preajustos", "Réamhshocruithe"]),
    ("eq.load", ["Load", "Charger", "Cargar", "Laden", "Carica", "Carrega", "Lódáil"]),
    ("eq.save", ["Save", "Enregistrer", "Guardar", "Speichern", "Salva", "Guarda", "Sábháil"]),
    ("eq.from_file", ["From EQF file…", "Depuis un fichier EQF…", "Desde un archivo EQF…", "Aus EQF-Datei…", "Da file EQF…", "Des d'un fitxer EQF…", "Ó chomhad EQF…"]),
    ("eq.to_file", ["To EQF file…", "Dans un fichier EQF…", "En un archivo EQF…", "In EQF-Datei…", "Su file EQF…", "En un fitxer EQF…", "Go comhad EQF…"]),
    ("eq.auto_preset", ["Auto-load preset for this track", "Préréglage automatique pour ce morceau", "Preajuste automático para esta pista", "Automatisches Preset für diesen Titel", "Preset automatico per questo brano", "Preajust automàtic per a esta pista", "Réamhshocrú uathoibríoch don rian seo"]),
    ("eq.delete_auto", ["Delete auto-load preset", "Supprimer le préréglage automatique", "Eliminar el preajuste automático", "Automatisches Preset löschen", "Elimina il preset automatico", "Elimina el preajust automàtic", "Scrios an réamhshocrú uathoibríoch"]),
    ("eq.reset", ["Reset (flat)", "Remettre à plat", "Restablecer (plano)", "Zurücksetzen (linear)", "Azzera (piatto)", "Restableix (pla)", "Athshocraigh (cothrom)"]),
    ("eq.enabled", ["Equalizer on", "Égaliseur activé", "Ecualizador activado", "Equalizer an", "Equalizzatore attivo", "Equalitzador activat", "Cothromóir air"]),
    ("eq.auto", ["Auto-load presets", "Préréglages automatiques", "Preajustes automáticos", "Presets automatisch laden", "Preset automatici", "Preajustos automàtics", "Réamhshocruithe uathoibríocha"]),
    ("pl.add_url", ["Add URL…", "Ajouter une adresse…", "Añadir URL…", "URL hinzufügen…", "Aggiungi URL…", "Afig una URL…", "Cuir URL leis…"]),
    ("pl.add_folder", ["Add folder…", "Ajouter un dossier…", "Añadir carpeta…", "Ordner hinzufügen…", "Aggiungi cartella…", "Afig una carpeta…", "Cuir fillteán leis…"]),
    ("pl.add_files", ["Add files…", "Ajouter des fichiers…", "Añadir archivos…", "Dateien hinzufügen…", "Aggiungi file…", "Afig fitxers…", "Cuir comhaid leis…"]),
    ("pl.remove_missing", ["Remove missing files", "Retirer les fichiers introuvables", "Quitar archivos inexistentes", "Fehlende Dateien entfernen", "Rimuovi i file mancanti", "Lleva els fitxers que falten", "Bain comhaid atá ar iarraidh"]),
    ("pl.remove_duplicates", ["Remove duplicates", "Retirer les doublons", "Quitar duplicados", "Duplikate entfernen", "Rimuovi i duplicati", "Lleva els duplicats", "Bain dúblaigh"]),
    ("pl.remove_selected", ["Remove selected", "Retirer la sélection", "Quitar la selección", "Auswahl entfernen", "Rimuovi i selezionati", "Lleva la selecció", "Bain an roghnúchán"]),
    ("pl.crop", ["Crop (keep selection)", "Rogner (garder la sélection)", "Recortar (conservar la selección)", "Zuschneiden (Auswahl behalten)", "Ritaglia (mantieni la selezione)", "Retalla (conserva la selecció)", "Bearr (coinnigh an roghnúchán)"]),
    ("pl.remove_all", ["Remove all", "Tout retirer", "Quitar todo", "Alle entfernen", "Rimuovi tutto", "Lleva-ho tot", "Bain gach rud"]),
    ("pl.select_all", ["Select all", "Tout sélectionner", "Seleccionar todo", "Alle auswählen", "Seleziona tutto", "Selecciona-ho tot", "Roghnaigh gach rud"]),
    ("pl.select_none", ["Select none", "Tout désélectionner", "No seleccionar nada", "Nichts auswählen", "Deseleziona tutto", "No seleccionis res", "Ná roghnaigh faic"]),
    ("pl.invert", ["Invert selection", "Inverser la sélection", "Invertir la selección", "Auswahl umkehren", "Inverti la selezione", "Inverteix la selecció", "Inbhéartaigh an roghnúchán"]),
    ("pl.sort", ["Sort", "Trier", "Ordenar", "Sortieren", "Ordina", "Ordena", "Sórtáil"]),
    ("pl.sort_title", ["Sort by title", "Trier par titre", "Ordenar por título", "Nach Titel sortieren", "Ordina per titolo", "Ordena per títol", "Sórtáil de réir teidil"]),
    ("pl.sort_filename", ["Sort by file name", "Trier par nom de fichier", "Ordenar por nombre de archivo", "Nach Dateiname sortieren", "Ordina per nome del file", "Ordena per nom de fitxer", "Sórtáil de réir ainm comhaid"]),
    ("pl.sort_path", ["Sort by path", "Trier par chemin", "Ordenar por ruta", "Nach Pfad sortieren", "Ordina per percorso", "Ordena per camí", "Sórtáil de réir conaire"]),
    ("pl.reverse", ["Reverse list", "Inverser la liste", "Invertir la lista", "Liste umkehren", "Inverti l'elenco", "Inverteix la llista", "Aisiompaigh an liosta"]),
    ("pl.randomize", ["Randomize list", "Mélanger la liste", "Mezclar la lista", "Liste mischen", "Mescola l'elenco", "Barreja la llista", "Randamaigh an liosta"]),
    ("pl.new", ["New playlist", "Nouvelle playlist", "Nueva lista", "Neue Wiedergabeliste", "Nuova playlist", "Llista nova", "Seinmliosta nua"]),
    ("pl.save", ["Save playlist…", "Enregistrer la playlist…", "Guardar la lista…", "Wiedergabeliste speichern…", "Salva la playlist…", "Guarda la llista…", "Sábháil an seinmliosta…"]),
    ("pl.load", ["Load playlist…", "Ouvrir une playlist…", "Abrir una lista…", "Wiedergabeliste laden…", "Apri una playlist…", "Obri una llista…", "Lódáil seinmliosta…"]),
    ("pl.play_item", ["Play", "Lire", "Reproducir", "Abspielen", "Riproduci", "Reproduix", "Seinn"]),
    ("jump.title", ["Jump to file", "Aller au morceau", "Saltar a la pista", "Zu Titel springen", "Vai al brano", "Salta a la pista", "Léim go rian"]),
    ("jump.prompt", ["Search:", "Rechercher :", "Buscar:", "Suchen:", "Cerca:", "Cerca:", "Cuardaigh:"]),
    ("url.title", ["Add URL", "Ajouter une adresse", "Añadir URL", "URL hinzufügen", "Aggiungi URL", "Afig una URL", "Cuir URL leis"]),
    ("url.prompt", ["Stream address (http://…):", "Adresse du flux (http://…) :", "Dirección del flujo (http://…):", "Stream-Adresse (http://…):", "Indirizzo del flusso (http://…):", "Adreça del flux (http://…):", "Seoladh an tsrutha (http://…):"]),
    ("dialog.ok", ["OK", "OK", "Aceptar", "OK", "OK", "D'acord", "OK"]),
    ("dialog.cancel", ["Cancel", "Annuler", "Cancelar", "Abbrechen", "Annulla", "Cancel·la", "Cealaigh"]),
    ("dialog.close", ["Close", "Fermer", "Cerrar", "Schließen", "Chiudi", "Tanca", "Dún"]),
    ("dialog.eq_files", ["EQ presets", "Préréglages EQ", "Preajustes EQ", "EQ-Presets", "Preset EQ", "Preajustos EQ", "Réamhshocruithe EQ"]),
    ("error.eqf", ["Could not read the EQ file", "Impossible de lire le fichier EQ", "No se pudo leer el archivo EQ", "EQ-Datei konnte nicht gelesen werden", "Impossibile leggere il file EQ", "No s'ha pogut llegir el fitxer EQ", "Níorbh fhéidir an comhad EQ a léamh"]),
    ("menu.bookmarks", ["Bookmarks", "Signets", "Marcadores", "Lesezeichen", "Segnalibri", "Marcadors", "Leabharmharcanna"]),
    ("bm.add", ["Add current as bookmark", "Ajouter le morceau en signet", "Añadir la pista actual a marcadores", "Aktuellen Titel als Lesezeichen", "Aggiungi il brano ai segnalibri", "Afig la pista actual als marcadors", "Cuir an rian reatha leis na leabharmharcanna"]),
    ("bm.remove", ["Remove", "Retirer", "Quitar", "Entfernen", "Rimuovi", "Lleva", "Bain"]),
    ("bm.clear", ["Clear bookmarks", "Effacer les signets", "Borrar los marcadores", "Lesezeichen löschen", "Cancella i segnalibri", "Esborra els marcadors", "Glan na leabharmharcanna"]),
    ("bm.empty", ["(no bookmarks)", "(aucun signet)", "(sin marcadores)", "(keine Lesezeichen)", "(nessun segnalibro)", "(cap marcador)", "(gan leabharmharcanna)"]),
    ("menu.crossfade", ["Crossfade", "Fondu enchaîné", "Fundido encadenado", "Überblendung", "Dissolvenza incrociata", "Fosa encadenada", "Traschéimniú"]),
    ("opt.off", ["Off", "Désactivé", "Desactivado", "Aus", "Disattivato", "Desactivat", "As"]),
    ("menu.replaygain", ["ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain"]),
    ("rg.track", ["Track gain", "Gain du morceau", "Ganancia de pista", "Titel-Verstärkung", "Guadagno del brano", "Guany de la pista", "Gnóthachan riain"]),
    ("rg.album", ["Album gain", "Gain de l'album", "Ganancia de álbum", "Album-Verstärkung", "Guadagno dell'album", "Guany de l'àlbum", "Gnóthachan albaim"]),
    ("rg.prevent_clipping", ["Prevent clipping", "Éviter la saturation", "Evitar la saturación", "Übersteuerung vermeiden", "Evita la distorsione", "Evita la saturació", "Seachain bearradh"]),
    ("rg.preamp", ["Preamp", "Préampli", "Preamplificador", "Vorverstärkung", "Preamplificatore", "Preamplificador", "Réamhaimplitheoir"]),
    ("vis.analyzer_style", ["Analyzer style", "Style de l'analyseur", "Estilo del analizador", "Analysator-Stil", "Stile dell'analizzatore", "Estil de l'analitzador", "Stíl an anailíseora"]),
    ("vis.normal", ["Normal", "Normal", "Normal", "Normal", "Normale", "Normal", "Gnáth"]),
    ("vis.fire", ["Fire", "Feu", "Fuego", "Feuer", "Fuoco", "Foc", "Tine"]),
    ("vis.line", ["Line", "Ligne", "Línea", "Linie", "Linea", "Línia", "Líne"]),
    ("vis.thick", ["Thick bands", "Barres épaisses", "Barras gruesas", "Breite Balken", "Barre spesse", "Barres gruixudes", "Barraí tiubha"]),
    ("vis.thin", ["Thin bands", "Barres fines", "Barras finas", "Schmale Balken", "Barre sottili", "Barres fines", "Barraí tanaí"]),
    ("vis.peaks", ["Show peaks", "Afficher les crêtes", "Mostrar picos", "Spitzen anzeigen", "Mostra i picchi", "Mostra els pics", "Taispeáin buaicphointí"]),
    ("vis.falloff", ["Analyzer falloff", "Chute des barres", "Caída de las barras", "Balkenabfall", "Caduta delle barre", "Caiguda de les barres", "Titim na mbarraí"]),
    ("vis.peak_falloff", ["Peaks falloff", "Chute des crêtes", "Caída de los picos", "Spitzenabfall", "Caduta dei picchi", "Caiguda dels pics", "Titim na mbuaicphointí"]),
    ("vis.speed0", ["Slowest", "Très lente", "Muy lenta", "Sehr langsam", "Molto lenta", "Molt lenta", "An-mhall"]),
    ("vis.speed1", ["Slower", "Lente", "Lenta", "Langsam", "Lenta", "Lenta", "Mall"]),
    ("vis.speed2", ["Moderate", "Moyenne", "Media", "Mittel", "Media", "Mitjana", "Measartha"]),
    ("vis.speed3", ["Faster", "Rapide", "Rápida", "Schnell", "Veloce", "Ràpida", "Tapa"]),
    ("vis.speed4", ["Fastest", "Très rapide", "Muy rápida", "Sehr schnell", "Molto veloce", "Molt ràpida", "An-tapa"]),
    ("vis.scope_style", ["Oscilloscope style", "Style de l'oscilloscope", "Estilo del osciloscopio", "Oszilloskop-Stil", "Stile dell'oscilloscopio", "Estil de l'oscil·loscopi", "Stíl an ascalascóip"]),
    ("vis.dots", ["Dots", "Points", "Puntos", "Punkte", "Punti", "Punts", "Poncanna"]),
    ("vis.lines", ["Lines", "Lignes", "Líneas", "Linien", "Linee", "Línies", "Línte"]),
    ("vis.solid", ["Solid", "Plein", "Sólido", "Gefüllt", "Pieno", "Ple", "Soladach"]),
    ("vis.refresh", ["Refresh rate", "Fréquence d'affichage", "Frecuencia de refresco", "Bildrate", "Frequenza di aggiornamento", "Freqüència de refresc", "Ráta athnuachana"]),
    ("vis.full", ["Full", "Pleine", "Completa", "Voll", "Piena", "Completa", "Iomlán"]),
    ("info.station", ["Station", "Station", "Emisora", "Sender", "Emittente", "Emissora", "Stáisiún"]),
    ("info.replaygain", ["ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain", "ReplayGain"]),
    ("mobile.phone_music", ["Music on this phone", "Musique du téléphone", "Música del teléfono", "Musik auf dem Telefon", "Musica del telefono", "Música del telèfon", "Ceol ar an bhfón seo"]),
    ("mobile.radios", ["Radios", "Radios", "Radios", "Radiosender", "Radio", "Ràdios", "Raidió"]),
    ("menu.library", ["Library", "Bibliothèque", "Biblioteca", "Bibliothek", "Libreria", "Biblioteca", "Leabharlann"]),
    ("lib.tab_playlists", ["Playlists", "Playlists", "Listas", "Listen", "Playlist", "Llistes", "Liostaí"]),
    ("lib.tab_liked", ["Liked", "Likés", "Me gusta", "Favoriten", "Preferiti", "M'agrada", "Is maith"]),
    ("lib.tab_albums", ["Albums", "Albums", "Álbumes", "Alben", "Album", "Àlbums", "Albaim"]),
    ("lib.tab_search", ["Search", "Chercher", "Buscar", "Suchen", "Cerca", "Cerca", "Cuardach"]),
    ("lib.tab_radios", ["Radios", "Radios", "Radios", "Radio", "Radio", "Ràdios", "Raidió"]),
    ("lib.liked_title", ["Liked Songs", "Titres likés", "Canciones que te gustan", "Lieblingssongs", "Brani che ti piacciono", "Cançons que t'agraden", "Amhráin a thaitníonn leat"]),
    ("lib.login", ["Log in to Spotify…", "Se connecter à Spotify…", "Iniciar sesión en Spotify…", "Bei Spotify anmelden…", "Accedi a Spotify…", "Inicia sessió a Spotify…", "Logáil isteach i Spotify…"]),
    ("lib.logout", ["Log out of Spotify", "Se déconnecter de Spotify", "Cerrar sesión de Spotify", "Von Spotify abmelden", "Esci da Spotify", "Tanca la sessió de Spotify", "Logáil amach as Spotify"]),
    ("lib.authorizing", ["Waiting for the browser…", "En attente du navigateur…", "Esperando al navegador…", "Warte auf den Browser…", "In attesa del browser…", "Esperant el navegador…", "Ag fanacht leis an mbrabhsálaí…"]),
    ("lib.connecting", ["Connecting to Spotify…", "Connexion à Spotify…", "Conectando con Spotify…", "Verbinde mit Spotify…", "Connessione a Spotify…", "Connectant amb Spotify…", "Ag ceangal le Spotify…"]),
    ("lib.loading", ["Loading…", "Chargement…", "Cargando…", "Laden…", "Caricamento…", "Carregant…", "Á lódáil…"]),
    ("lib.empty", ["Nothing here", "Rien ici", "Nada aquí", "Nichts hier", "Niente qui", "Res ací", "Faic anseo"]),
    ("lib.search_prompt", ["Search Spotify", "Chercher sur Spotify", "Buscar en Spotify", "Spotify durchsuchen", "Cerca su Spotify", "Cerca a Spotify", "Cuardaigh Spotify"]),
    ("lib.radio_prompt", ["Radio name or genre", "Nom ou genre de radio", "Nombre o género de radio", "Sendername oder Genre", "Nome o genere della radio", "Nom o gènere de ràdio", "Ainm nó seánra raidió"]),
    ("lib.back", ["‹ Back", "‹ Retour", "‹ Atrás", "‹ Zurück", "‹ Indietro", "‹ Arrere", "‹ Siar"]),
    ("lib.add", ["Add", "Ajouter", "Añadir", "Hinzufügen", "Aggiungi", "Afig", "Cuir leis"]),
    ("menu.milkdrop", ["Visualization window", "Fenêtre de visualisation", "Ventana de visualización", "Visualisierungsfenster", "Finestra di visualizzazione", "Finestra de visualització", "Fuinneog léirshamhlaithe"]),
    ("milk.title", ["Visualization", "Visualisation", "Visualización", "Visualisierung", "Visualizzazione", "Visualització", "Léirshamhlú"]),
    ("milk.next", ["Next preset", "Preset suivant", "Preset siguiente", "Nächstes Preset", "Preset successivo", "Preset següent", "An chéad réamhshocrú eile"]),
    ("milk.next_cut", ["Next preset (hard cut)", "Preset suivant (coupe franche)", "Preset siguiente (corte seco)", "Nächstes Preset (harter Schnitt)", "Preset successivo (stacco netto)", "Preset següent (tall sec)", "An chéad réamhshocrú eile (gearradh glan)"]),
    ("milk.previous", ["Previous preset", "Preset précédent", "Preset anterior", "Vorheriges Preset", "Preset precedente", "Preset anterior", "An réamhshocrú roimhe seo"]),
    ("milk.random", ["Random order", "Ordre aléatoire", "Orden aleatorio", "Zufällige Reihenfolge", "Ordine casuale", "Orde aleatori", "Ord randamach"]),
    ("milk.lock", ["Lock current preset", "Verrouiller le preset", "Bloquear el preset", "Preset festhalten", "Blocca il preset", "Bloca el preset", "Glasáil an réamhshocrú"]),
    ("milk.locked", ["Preset locked", "Preset verrouillé", "Preset bloqueado", "Preset festgehalten", "Preset bloccato", "Preset blocat", "Réamhshocrú glasáilte"]),
    ("milk.unlocked", ["Preset unlocked", "Preset déverrouillé", "Preset desbloqueado", "Preset freigegeben", "Preset sbloccato", "Preset desblocat", "Réamhshocrú díghlasáilte"]),
    ("milk.choose", ["Choose a preset…", "Choisir un preset…", "Elegir un preset…", "Preset auswählen…", "Scegli un preset…", "Tria un preset…", "Roghnaigh réamhshocrú…"]),
    ("milk.search", ["Search presets:", "Chercher un preset :", "Buscar presets:", "Presets suchen:", "Cerca preset:", "Cerca presets:", "Cuardaigh réamhshocruithe:"]),
    ("milk.show", ["Show", "Afficher", "Mostrar", "Zeigen", "Mostra", "Mostra", "Taispeáin"]),
    ("milk.auto", ["Change preset every", "Changer de preset toutes les", "Cambiar de preset cada", "Preset wechseln alle", "Cambia preset ogni", "Canvia de preset cada", "Athraigh an réamhshocrú gach"]),
    ("milk.never", ["Never", "Jamais", "Nunca", "Nie", "Mai", "Mai", "Riamh"]),
    ("milk.blend", ["Blend time", "Durée du fondu", "Duración del fundido", "Überblendzeit", "Durata della dissolvenza", "Durada de la fosa", "Am cumaisc"]),
    ("milk.cut", ["Hard cut", "Coupe franche", "Corte seco", "Harter Schnitt", "Stacco netto", "Tall sec", "Gearradh glan"]),
    ("milk.fullscreen", ["Full screen", "Plein écran", "Pantalla completa", "Vollbild", "Schermo intero", "Pantalla completa", "Lánscáileán"]),
    ("milk.download", ["Download the preset pack (Cream of the Crop, about 30 MB)", "Télécharger le pack de presets (Cream of the Crop, 30 Mo environ)", "Descargar el paquete de presets (Cream of the Crop, unos 30 MB)", "Preset-Paket herunterladen (Cream of the Crop, etwa 30 MB)", "Scarica il pacchetto di preset (Cream of the Crop, circa 30 MB)", "Baixa el paquet de presets (Cream of the Crop, uns 30 MB)", "Íoslódáil an pacáiste réamhshocruithe (Cream of the Crop, thart ar 30 MB)"]),
    ("milk.downloading", ["Downloading presets…", "Téléchargement des presets…", "Descargando presets…", "Presets werden geladen…", "Download dei preset…", "Baixant presets…", "Ag íoslódáil réamhshocruithe…"]),
    ("milk.extracting", ["Unpacking presets…", "Décompression des presets…", "Descomprimiendo presets…", "Presets werden entpackt…", "Estrazione dei preset…", "Descomprimint presets…", "Ag díphacáil réamhshocruithe…"]),
    ("milk.ready", ["presets installed", "presets installés", "presets instalados", "Presets installiert", "preset installati", "presets instal·lats", "réamhshocrú suiteáilte"]),
    ("milk.failed", ["Preset download failed", "Échec du téléchargement des presets", "Error al descargar los presets", "Download der Presets fehlgeschlagen", "Download dei preset non riuscito", "Ha fallat la baixada dels presets", "Theip ar íoslódáil na réamhshocruithe"]),
    ("milk.open_folder", ["Open the presets folder", "Ouvrir le dossier des presets", "Abrir la carpeta de presets", "Preset-Ordner öffnen", "Apri la cartella dei preset", "Obri la carpeta de presets", "Oscail fillteán na réamhshocruithe"]),
    ("milk.rescan", ["Reload presets", "Recharger les presets", "Recargar presets", "Presets neu laden", "Ricarica i preset", "Torna a carregar els presets", "Athlódáil na réamhshocruithe"]),
    ("milk.no_gpu", ["No compatible graphics card (Vulkan, Metal or DirectX 12): the visualization window is unavailable.", "Aucune carte graphique compatible (Vulkan, Metal ou DirectX 12) : la fenêtre de visualisation est indisponible.", "Ninguna tarjeta gráfica compatible (Vulkan, Metal o DirectX 12): la ventana de visualización no está disponible.", "Keine kompatible Grafikkarte (Vulkan, Metal oder DirectX 12): Das Visualisierungsfenster ist nicht verfügbar.", "Nessuna scheda grafica compatibile (Vulkan, Metal o DirectX 12): la finestra di visualizzazione non è disponibile.", "Cap targeta gràfica compatible (Vulkan, Metal o DirectX 12): la finestra de visualització no està disponible.", "Níl cárta grafaice comhoiriúnach ann (Vulkan, Metal nó DirectX 12): níl fuinneog an léirshamhlaithe ar fáil."]),
    ("milk.presets", ["Presets", "Presets", "Presets", "Presets", "Preset", "Presets", "Réamhshocruithe"]),
    ("milk.exit", ["Leave the visualization", "Quitter la visualisation", "Salir de la visualización", "Visualisierung beenden", "Esci dalla visualizzazione", "Ix de la visualització", "Fág an léirshamhlú"]),
    ("milk.hint", ["Swipe: next / previous · double tap: leave · long press: menu", "Glisser : suivant / précédent · double-tap : sortir · appui long : menu", "Desliza: siguiente / anterior · doble toque: salir · pulsación larga: menú", "Wischen: weiter / zurück · Doppeltippen: beenden · lange drücken: Menü", "Scorri: successivo / precedente · doppio tocco: esci · pressione lunga: menu", "Llisca: següent / anterior · doble toc: eixir · pulsació llarga: menú", "Svaidhpeáil: ar aghaidh / siar · tapáil faoi dhó: fág · brú fada: roghchlár"]),
    ("milk.gpu", ["Draw with the graphics card (restart)", "Affichage par la carte graphique (redémarrer)", "Dibujar con la tarjeta gráfica (reiniciar)", "Mit der Grafikkarte zeichnen (Neustart)", "Disegna con la scheda grafica (riavvia)", "Dibuixa amb la targeta gràfica (reinicia)", "Tarraing leis an gcárta grafaice (atosaigh)"]),
    ("skins.title", ["Skins", "Skins", "Skins", "Skins", "Skin", "Aparences", "Craicne"]),
    ("menu.skin_browser", ["Skin browser…", "Navigateur de skins…", "Explorador de skins…", "Skin-Browser…", "Esplora skin…", "Navegador d'aparences…", "Brabhsálaí craiceann…"]),
    ("skins.museum", ["Museum", "Musée", "Museo", "Museum", "Museo", "Museu", "Músaem"]),
    ("skins.mine", ["My skins", "Mes skins", "Mis skins", "Meine", "Le mie", "Les meues", "Mo chuid"]),
    ("skins.search", ["Search skins", "Chercher un skin", "Buscar skins", "Skins suchen", "Cerca skin", "Cerca aparences", "Cuardaigh craicne"]),
    ("skins.open", ["Open…", "Ouvrir…", "Abrir…", "Öffnen…", "Apri…", "Obri…", "Oscail…"]),
    ("skins.loading", ["Loading…", "Chargement…", "Cargando…", "Laden…", "Caricamento…", "Carregant…", "Á lódáil…"]),
    ("skins.downloading", ["Downloading…", "Téléchargement…", "Descargando…", "Wird geladen…", "Download…", "Baixant…", "Á íoslódáil…"]),
    ("skins.applied", ["Applied", "Appliqué", "Aplicado", "Angewendet", "Applicata", "Aplicada", "Curtha i bhfeidhm"]),
    ("skins.default", ["Default skin", "Skin par défaut", "Skin por defecto", "Standard-Skin", "Skin predefinita", "Aparença per defecte", "Craiceann réamhshocraithe"]),
    ("skins.credit", ["Skins from the Winamp Skin Museum (skins.webamp.org)", "Skins du Winamp Skin Museum (skins.webamp.org)", "Skins del Winamp Skin Museum (skins.webamp.org)", "Skins aus dem Winamp Skin Museum (skins.webamp.org)", "Skin dal Winamp Skin Museum (skins.webamp.org)", "Aparences del Winamp Skin Museum (skins.webamp.org)", "Craicne ón Winamp Skin Museum (skins.webamp.org)"]),
    ("skins.error", ["Skin Museum unreachable", "Musée des skins injoignable", "Museo de skins inaccesible", "Skin-Museum nicht erreichbar", "Museo delle skin irraggiungibile", "Museu d'aparences inaccessible", "Ní féidir teacht ar an músaem craiceann"]),
    ("skins.none", ["No skin found", "Aucun skin trouvé", "Ningún skin encontrado", "Kein Skin gefunden", "Nessuna skin trovata", "Cap aparença trobada", "Níor aimsíodh craiceann"]),
    ("mobile.milk", ["Full-screen visualization", "Visualisation plein écran", "Visualización a pantalla completa", "Vollbild-Visualisierung", "Visualizzazione a schermo intero", "Visualització a pantalla completa", "Léirshamhlú lánscáileáin"]),
    ("error.skin", ["Could not load the skin", "Impossible de charger le skin", "No se pudo cargar el skin", "Skin konnte nicht geladen werden", "Impossibile caricare la skin", "No s'ha pogut carregar l'aparença", "Níorbh fhéidir an craiceann a lódáil"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_fallback() {
        assert_eq!(Lang::from_tag("fr-FR"), Some(Lang::Fr));
        assert_eq!(Lang::from_tag("ca-ES-valencia"), Some(Lang::Va));
        assert_eq!(Lang::from_tag("ga_IE"), Some(Lang::Ga));
        assert_eq!(Lang::from_tag("ja"), None);
        set_lang(Lang::De);
        assert_eq!(t("menu.next"), "Weiter");
        assert_eq!(t("inconnu.cle"), "inconnu.cle");
        set_lang(Lang::En);
    }

    #[test]
    fn every_entry_has_all_languages() {
        for (k, tr) in TABLE {
            for (i, s) in tr.iter().enumerate() {
                assert!(!s.is_empty(), "{k} manque en {}", Lang::ALL[i].code());
            }
        }
    }
}
