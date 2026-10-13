//! Appels au système Android par JNI (sans code Java) : permission de lire la musique, fin de l'activité.

use std::path::{Path, PathBuf};

use jni::JavaVM;
use jni::objects::{JObject, JValue};
use winit::platform::android::activity::AndroidApp;

fn with_env<T>(app: &AndroidApp, f: impl FnOnce(&mut jni::JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
    let r = f(&mut env, &activity);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("JNI : {e}");
            None
        }
    }
}

fn sdk_int(env: &mut jni::JNIEnv) -> jni::errors::Result<i32> {
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()
}

/// Nom de la permission de lecture de la musique selon la version d'Android.
fn audio_permission(env: &mut jni::JNIEnv) -> jni::errors::Result<&'static str> {
    Ok(if sdk_int(env)? >= 33 { "android.permission.READ_MEDIA_AUDIO" } else { "android.permission.READ_EXTERNAL_STORAGE" })
}

/// La permission de lire la musique est-elle accordée ?
pub fn has_music_permission(app: &AndroidApp) -> bool {
    with_env(app, |env, activity| {
        let perm = audio_permission(env)?;
        let name = env.new_string(perm)?;
        let r = env.call_method(activity, "checkSelfPermission", "(Ljava/lang/String;)I", &[JValue::Object(&name)])?.i()?;
        Ok(r == 0)
    })
    .unwrap_or(false)
}

/// Demande la permission (boîte du système ; la réponse arrive au retour dans l'appli).
pub fn request_music_permission(app: &AndroidApp) {
    with_env(app, |env, activity| {
        let perm = audio_permission(env)?;
        let name = env.new_string(perm)?;
        let arr = env.new_object_array(1, "java/lang/String", &name)?;
        env.call_method(activity, "requestPermissions", "([Ljava/lang/String;I)V", &[JValue::Object(&arr), JValue::Int(1)])?;
        Ok(())
    });
}

/// Ferme l'activité.
pub fn finish(app: &AndroidApp) {
    with_env(app, |env, activity| {
        env.call_method(activity, "finish", "()V", &[])?;
        Ok(())
    });
}

/// Dossier de stockage partagé (« /storage/emulated/0 »).
pub fn shared_storage() -> PathBuf {
    std::env::var_os("EXTERNAL_STORAGE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/storage/emulated/0"))
}

/// Dossiers où chercher la musique du téléphone.
pub fn music_dirs() -> Vec<PathBuf> {
    let root = shared_storage();
    ["Music", "Download", "Podcasts", "Audiobooks", "Recordings"].iter().map(|d| root.join(d)).filter(|p| p.is_dir()).collect()
}

/// Dossier de données privé de l'appli.
pub fn data_dir(app: &AndroidApp) -> PathBuf {
    app.internal_data_path().unwrap_or_else(|| Path::new("/data/local/tmp/alpagamp").to_path_buf())
}
