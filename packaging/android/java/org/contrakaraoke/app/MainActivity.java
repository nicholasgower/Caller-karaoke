package org.contrakaraoke.app;

import android.app.NativeActivity;

/**
 * Native activity whose class loader owns the Rust library.
 *
 * The `tts` crate's `JNI_OnLoad` looks up `rs.tts.Bridge` with `FindClass`.
 * Loading the library from an app class (rather than the framework's
 * NativeActivity) makes that lookup use the app class loader, which includes
 * classes.dex.
 */
public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("contra_app");
    }
}
