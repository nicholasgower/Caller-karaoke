package rs.tts;

import android.speech.tts.TextToSpeech;
import android.speech.tts.UtteranceProgressListener;

/**
 * JNI bridge required by the Rust `tts` crate's Android backend.
 *
 * The crate's `JNI_OnLoad` looks this class up by name, so it must be packaged
 * into the APK's classes.dex. Keep the package and native method signatures in
 * sync with `tts` 0.26.x; see the crate's `examples/android` directory.
 */
public class Bridge extends UtteranceProgressListener implements TextToSpeech.OnInitListener {
    public int backendId;

    public Bridge(int backendId) {
        this.backendId = backendId;
    }

    public native void onInit(int status);

    public native void onStart(String utteranceId);

    public native void onStop(String utteranceId, Boolean interrupted);

    public native void onDone(String utteranceId);

    public native void onError(String utteranceId);
}
