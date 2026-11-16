package org.contrakaraoke.app;

import android.app.NativeActivity;
import android.os.Build;
import android.os.Bundle;
import android.view.WindowInsets;

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

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Feed the actual status/navigation bar insets to Rust. The native
        // content rectangle also includes the gesture area on some devices,
        // which leaves too much empty space above the tabs.
        getWindow().getDecorView().setOnApplyWindowInsetsListener((view, insets) -> {
            publishSystemBarInsets(insets);
            return view.onApplyWindowInsets(insets);
        });
        getWindow().getDecorView().requestApplyInsets();
        publishSystemBarInsets(getWindow().getDecorView().getRootWindowInsets());
    }

    private static void publishSystemBarInsets(WindowInsets insets) {
        if (insets == null) {
            return;
        }
        if (Build.VERSION.SDK_INT >= 30) {
            android.graphics.Insets bars = insets.getInsets(WindowInsets.Type.systemBars());
            nativeSetSystemBarInsets(bars.left, bars.top, bars.right, bars.bottom);
        } else {
            nativeSetSystemBarInsets(
                    insets.getSystemWindowInsetLeft(),
                    insets.getSystemWindowInsetTop(),
                    insets.getSystemWindowInsetRight(),
                    insets.getSystemWindowInsetBottom());
        }
    }

    private static native void nativeSetSystemBarInsets(
            int left, int top, int right, int bottom);
}
