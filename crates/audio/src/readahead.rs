//! Lecture anticipée des fichiers locaux (0.6.2, ADR 0022).
//!
//! Le décodeur lit le fichier par petits blocs (32 Ko au plus) au moment où il en a besoin, et le moteur ne
//! garde que 0,2 s d'avance : sur un disque lent (disque externe qui se réveille, disque occupé), une lecture de
//! 1 à 2 s coupait la musique. Ici, un fil lit le fichier **en avance** (jusqu'à 16 Mo devant la position du
//! décodeur, soit le fichier entier pour la plupart des morceaux) ; le décodeur ne fait que copier de la mémoire.
//! Un saut dans le morceau (seek) en dehors de ce qui est déjà lu relance la lecture à la nouvelle position.

use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};
use symphonia::core::io::MediaSource;

/// Taille d'une lecture du fil.
const CHUNK: usize = 256 * 1024;
/// Avance maximale sur la position du décodeur.
pub const AHEAD: u64 = 16 * 1024 * 1024;
/// Ce qu'on garde derrière la position du décodeur (petits retours en arrière sans relire le disque).
const BEHIND: u64 = 2 * 1024 * 1024;

struct State {
    /// Position dans le fichier du premier octet de `buf`.
    base: u64,
    buf: VecDeque<u8>,
    eof: bool,
    error: Option<String>,
    /// Position du décodeur (pour savoir jusqu'où lire d'avance).
    pos: u64,
    /// Le décodeur veut des données ailleurs : la lecture repart de là.
    restart: Option<u64>,
    /// Numéro de « tranche » : une lecture commencée avant une relance est jetée.
    generation: u64,
    closed: bool,
}

type Shared = Arc<(Mutex<State>, Condvar)>;

/// Une source lue d'avance par un fil.
pub struct ReadAhead {
    shared: Shared,
    pos: u64,
    len: u64,
}

impl ReadAhead {
    /// Ouvre un fichier local.
    pub fn open(path: &Path) -> io::Result<ReadAhead> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        ReadAhead::new(file, len)
    }

    /// Lit `source` (de longueur `len`) d'avance dans un fil.
    pub fn new<R: Read + Seek + Send + 'static>(source: R, len: u64) -> io::Result<ReadAhead> {
        let shared: Shared = Arc::new((
            Mutex::new(State { base: 0, buf: VecDeque::new(), eof: false, error: None, pos: 0, restart: None, generation: 0, closed: false }),
            Condvar::new(),
        ));
        let sh = shared.clone();
        std::thread::Builder::new().name("alpagamp-lecture".into()).spawn(move || fill(source, sh))?;
        Ok(ReadAhead { shared, pos: 0, len })
    }

    /// Octets déjà lus d'avance à partir de la position courante (pour les tests et le diagnostic).
    pub fn buffered_ahead(&self) -> u64 {
        let st = self.shared.0.lock();
        (st.base + st.buf.len() as u64).saturating_sub(self.pos)
    }
}

/// Le fil de lecture.
fn fill<R: Read + Seek>(mut source: R, shared: Shared) {
    let (m, cv) = &*shared;
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let (offset, generation) = {
            let mut st = m.lock();
            loop {
                if st.closed {
                    return;
                }
                if let Some(p) = st.restart.take() {
                    st.base = p;
                    st.buf.clear();
                    st.eof = false;
                    st.error = None;
                    st.generation += 1;
                }
                let end = st.base + st.buf.len() as u64;
                if !st.eof && st.error.is_none() && end < st.pos + AHEAD {
                    break (end, st.generation);
                }
                cv.wait(&mut st);
            }
        };
        let r = source.seek(SeekFrom::Start(offset)).and_then(|_| source.read(&mut chunk));
        let mut st = m.lock();
        if st.generation != generation || st.restart.is_some() {
            continue;
        }
        match r {
            Ok(0) => st.eof = true,
            Ok(n) => st.buf.extend(&chunk[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => st.error = Some(e.to_string()),
        }
        // On ne garde qu'un peu de ce que le décodeur a déjà lu.
        let behind = st.pos.saturating_sub(st.base);
        if behind > BEHIND {
            let drop = ((behind - BEHIND) as usize).min(st.buf.len());
            st.buf.drain(..drop);
            st.base += drop as u64;
        }
        cv.notify_all();
    }
}

impl Read for ReadAhead {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() || self.pos >= self.len {
            return Ok(0);
        }
        let (m, cv) = &*self.shared;
        let mut st = m.lock();
        st.pos = self.pos;
        loop {
            let end = st.base + st.buf.len() as u64;
            if st.restart.is_none() {
                if self.pos >= st.base && self.pos < end {
                    let off = (self.pos - st.base) as usize;
                    let n = out.len().min(st.buf.len() - off);
                    let (a, b) = st.buf.as_slices();
                    let mut copied = 0;
                    if off < a.len() {
                        let k = n.min(a.len() - off);
                        out[..k].copy_from_slice(&a[off..off + k]);
                        copied = k;
                    }
                    if copied < n {
                        let boff = off + copied - a.len();
                        out[copied..n].copy_from_slice(&b[boff..boff + n - copied]);
                    }
                    self.pos += n as u64;
                    st.pos = self.pos;
                    cv.notify_all();
                    return Ok(n);
                }
                if self.pos == end {
                    if st.eof {
                        return Ok(0);
                    }
                    if let Some(e) = st.error.take() {
                        return Err(io::Error::other(e));
                    }
                } else {
                    // Hors de ce qui est lu : la lecture repart d'ici.
                    st.restart = Some(self.pos);
                }
            }
            cv.notify_all();
            cv.wait(&mut st);
        }
    }
}

impl Seek for ReadAhead {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let target = match to {
            SeekFrom::Start(p) => p as i128,
            SeekFrom::End(d) => self.len as i128 + d as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if target < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "position négative"));
        }
        self.pos = target as u64;
        let (m, cv) = &*self.shared;
        m.lock().pos = self.pos;
        cv.notify_all();
        Ok(self.pos)
    }
}

impl MediaSource for ReadAhead {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

impl Drop for ReadAhead {
    fn drop(&mut self) {
        let (m, cv) = &*self.shared;
        m.lock().closed = true;
        cv.notify_all();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{Duration, Instant};

    /// Un « disque lent » : chaque lecture de 64 Ko au plus prend `delay` (comme un disque externe qui
    /// peine), et une fois sur `stall_every` elle bloque `stall`.
    pub struct SlowDisk {
        pub inner: Cursor<Vec<u8>>,
        pub delay: Duration,
        pub stall: Duration,
        pub stall_every: u32,
        pub reads: u32,
    }

    impl Read for SlowDisk {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            std::thread::sleep(if self.stall_every > 0 && self.reads % self.stall_every == 0 { self.stall } else { self.delay });
            let n = out.len().min(64 * 1024);
            self.inner.read(&mut out[..n])
        }
    }

    impl Seek for SlowDisk {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            self.inner.seek(to)
        }
    }

    fn data(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 31 % 251) as u8).collect()
    }

    #[test]
    fn reads_ahead_and_seeks() {
        let d = data(3 * CHUNK + 1234);
        let mut r = ReadAhead::new(Cursor::new(d.clone()), d.len() as u64).unwrap();
        let mut all = Vec::new();
        r.read_to_end(&mut all).unwrap();
        assert_eq!(all, d);
        // Retour en arrière (dans ce qui est gardé), puis saut à la fin.
        r.seek(SeekFrom::Start(100)).unwrap();
        let mut b = [0u8; 10];
        r.read_exact(&mut b).unwrap();
        assert_eq!(&b, &d[100..110]);
        assert_eq!(r.seek(SeekFrom::End(-10)).unwrap(), d.len() as u64 - 10);
        let mut tail = Vec::new();
        r.read_to_end(&mut tail).unwrap();
        assert_eq!(tail, &d[d.len() - 10..]);
        assert_eq!(r.read(&mut b).unwrap(), 0);
    }

    #[test]
    fn far_seek_restarts_reading_there() {
        // Un fichier plus grand que la fenêtre d'avance : un saut lointain relance la lecture.
        let len = (AHEAD + 4 * BEHIND) as usize + 999;
        let d = data(len);
        let mut r = ReadAhead::new(Cursor::new(d.clone()), len as u64).unwrap();
        let mut b = vec![0u8; 5000];
        r.read_exact(&mut b).unwrap();
        assert_eq!(&b[..], &d[..5000]);
        let far = len as u64 - 6000;
        r.seek(SeekFrom::Start(far)).unwrap();
        r.read_exact(&mut b).unwrap();
        assert_eq!(&b[..], &d[far as usize..far as usize + 5000]);
        r.seek(SeekFrom::Start(10)).unwrap();
        r.read_exact(&mut b).unwrap();
        assert_eq!(&b[..], &d[10..5010]);
    }

    /// Le cas du 15/10 : un disque qui bloque 1,5 s régulièrement. Une fois le début lu, le décodeur ne
    /// doit plus jamais attendre le disque.
    #[test]
    fn slow_disk_does_not_block_the_reader() {
        let d = data(2 * 1024 * 1024);
        let disk = SlowDisk { inner: Cursor::new(d.clone()), delay: Duration::from_millis(5), stall: Duration::from_millis(1500), stall_every: 12, reads: 0 };
        let mut r = ReadAhead::new(disk, d.len() as u64).unwrap();
        // Le fichier entier (2 Mo) arrive en mémoire, malgré les blocages.
        let t = Instant::now();
        while r.buffered_ahead() < d.len() as u64 {
            assert!(t.elapsed() < Duration::from_secs(20), "lecture d'avance trop lente");
            std::thread::sleep(Duration::from_millis(20));
        }
        // Le décodeur lit ensuite par blocs de 4 Ko : aucune lecture ne doit attendre.
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        let mut worst = Duration::ZERO;
        loop {
            let t = Instant::now();
            let n = r.read(&mut buf).unwrap();
            worst = worst.max(t.elapsed());
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }
        assert_eq!(out, d);
        assert!(worst < Duration::from_millis(50), "une lecture a attendu {worst:?}");
    }
}
