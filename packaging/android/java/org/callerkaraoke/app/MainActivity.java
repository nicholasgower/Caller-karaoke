package org.callerkaraoke.app;

import android.app.NativeActivity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageManager;
import android.database.Cursor;
import android.media.MediaMetadata;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.provider.MediaStore;
import android.provider.OpenableColumns;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.EditText;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

/**
 * Native activity whose class loader owns the Rust library.
 *
 * The `tts` crate's `JNI_OnLoad` looks up `rs.tts.Bridge` with `FindClass`.
 * Loading the library from an app class (rather than the framework's
 * NativeActivity) makes that lookup use the app class loader, which includes
 * classes.dex.
 *
 * <p>The activity also owns the media session. Android delivers headset,
 * Bluetooth, lock-screen, and system media controls to a {@link MediaSession},
 * and the transport itself lives in Rust. Commands therefore cross the JNI
 * boundary through {@link #nativeMediaCommand}, while playback state and the
 * loaded dance and tune come back through the static setters at the bottom of
 * this class.
 */
public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("caller_app");
    }

    /** Command codes; they must match `MediaCommand` in `media_keys.rs`. */
    private static final int COMMAND_PLAY = 1;
    private static final int COMMAND_PAUSE = 2;
    private static final int COMMAND_TOGGLE = 3;

    private static final String CHANNEL_ID = "playback";
    private static final int NOTIFICATION_ID = 1;
    private static final int PERMISSION_REQUEST_NOTIFICATIONS = 1;
    private static final int REQUEST_IMPORT = 2;
    /** Largest file the document picker will read, in bytes. */
    private static final int MAX_IMPORT_BYTES = 4 * 1024 * 1024;
    private static final String ACTION_PLAY = "org.callerkaraoke.app.action.PLAY";
    private static final String ACTION_PAUSE = "org.callerkaraoke.app.action.PAUSE";
    private static final String ACTION_TOGGLE = "org.callerkaraoke.app.action.TOGGLE";

    private static MediaSession mediaSession;
    private static Context appContext;
    /** The live activity, used to start the document picker and the share sheet. */
    private static MainActivity appActivity;
    /** Import kind requested with {@link #openDocument}; 1 is ABC, 2 is dance JSON. */
    private static int importKind;
    private static BroadcastReceiver notificationReceiver;
    private static boolean playing;
    private static String title = "Caller Karaoke";
    private static String artist = "";
    private static String album = "Caller Karaoke";
    /**
     * The invisible field that gives this native activity a working soft keyboard.
     *
     * <p>A {@link NativeActivity}'s content view has no input connection, so
     * {@link InputMethodManager#showSoftInput} ignores it. Focusing this field
     * gives the IME somewhere to send text; {@link BridgeInputConnection}
     * forwards that text to Rust.
     */
    private static EditText textInput;

    /** Key codes for {@link #nativeImeKey}; they must match `KEY_*` in `android_ime.rs`. */
    private static final int KEY_BACKSPACE = 1;
    private static final int KEY_DELETE = 2;
    private static final int KEY_ENTER = 3;
    private static final int KEY_LEFT = 4;
    private static final int KEY_RIGHT = 5;
    private static final int KEY_UP = 6;
    private static final int KEY_DOWN = 7;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        appContext = getApplicationContext();
        appActivity = this;

        // Publish this class to Rust before the native event loop starts using
        // it. The loop's thread is attached to the VM without the app class
        // loader, so FindClass there cannot see this activity (see
        // media_keys.rs).
        nativeAttachActivity();
        createTextInput();

        // Feed the actual status/navigation bar insets to Rust. The native
        // content rectangle also includes the gesture area on some devices,
        // which leaves too much empty space above the tabs.
        getWindow().getDecorView().setOnApplyWindowInsetsListener((view, insets) -> {
            publishSystemBarInsets(insets);
            return view.onApplyWindowInsets(insets);
        });
        getWindow().getDecorView().requestApplyInsets();
        publishSystemBarInsets(getWindow().getDecorView().getRootWindowInsets());

        startMediaSession();
        requestNotificationPermission();
    }

    @Override
    protected void onDestroy() {
        if (appActivity == this) {
            appActivity = null;
        }
        if (notificationReceiver != null) {
            unregisterReceiver(notificationReceiver);
            notificationReceiver = null;
        }
        if (mediaSession != null) {
            mediaSession.setActive(false);
            mediaSession.release();
            mediaSession = null;
        }
        NotificationManager manager =
                (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager != null) {
            manager.cancel(NOTIFICATION_ID);
        }
        super.onDestroy();
    }

    /**
     * Register the session and route every transport button to Rust.
     *
     * The session stays active while the activity is paused so the lock screen
     * and a paired headset can still control a running session.
     */
    private void startMediaSession() {
        if (mediaSession != null) {
            return;
        }
        MediaSession session = new MediaSession(this, "CallerKaraoke");
        session.setCallback(new MediaSession.Callback() {
            @Override
            public void onPlay() {
                sendMediaCommand(COMMAND_PLAY);
            }

            @Override
            public void onPause() {
                sendMediaCommand(COMMAND_PAUSE);
            }

            @Override
            public void onStop() {
                sendMediaCommand(COMMAND_PAUSE);
            }

            @Override
            public boolean onMediaButtonEvent(Intent intent) {
                KeyEvent event = intent.getParcelableExtra(Intent.EXTRA_KEY_EVENT);
                if (event != null && event.getAction() == KeyEvent.ACTION_DOWN) {
                    switch (event.getKeyCode()) {
                        case KeyEvent.KEYCODE_MEDIA_PLAY:
                            sendMediaCommand(COMMAND_PLAY);
                            return true;
                        case KeyEvent.KEYCODE_MEDIA_PAUSE:
                            sendMediaCommand(COMMAND_PAUSE);
                            return true;
                        case KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE:
                        case KeyEvent.KEYCODE_HEADSETHOOK:
                            sendMediaCommand(COMMAND_TOGGLE);
                            return true;
                        default:
                            break;
                    }
                }
                return super.onMediaButtonEvent(intent);
            }
        });
        session.setSessionActivity(contentIntent());
        mediaSession = session;
        // minSdk 26: the platform delivers media buttons to every active
        // session, so no FLAG_HANDLES_* flags are needed here.
        session.setActive(true);

        registerNotificationReceiver();
        publishMetadata();
        publishPlaybackState();
        showNotification();
    }

    /**
     * Ask for the notification permission on Android 13+.
     *
     * Refusing only hides the notification: the media session is independent
     * of it, so headset, Bluetooth, lock-screen, and system controls still work.
     */
    private void requestNotificationPermission() {
        if (Build.VERSION.SDK_INT < 33) {
            return;
        }
        if (checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(
                    new String[] {android.Manifest.permission.POST_NOTIFICATIONS},
                    PERMISSION_REQUEST_NOTIFICATIONS);
        }
    }

    /** Route a notification action back into the transport. */
    private void registerNotificationReceiver() {
        if (notificationReceiver != null) {
            return;
        }
        notificationReceiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context context, Intent intent) {
                String action = intent.getAction();
                if (ACTION_PLAY.equals(action)) {
                    sendMediaCommand(COMMAND_PLAY);
                } else if (ACTION_PAUSE.equals(action)) {
                    sendMediaCommand(COMMAND_PAUSE);
                } else if (ACTION_TOGGLE.equals(action)) {
                    sendMediaCommand(COMMAND_TOGGLE);
                }
            }
        };
        IntentFilter filter = new IntentFilter();
        filter.addAction(ACTION_PLAY);
        filter.addAction(ACTION_PAUSE);
        filter.addAction(ACTION_TOGGLE);
        if (Build.VERSION.SDK_INT >= 33) {
            // The notification's PendingIntents run with this app's identity,
            // so an unexported receiver still receives them; no other app can.
            registerReceiver(notificationReceiver, filter, Context.RECEIVER_NOT_EXPORTED);
        } else {
            registerReceiver(notificationReceiver, filter);
        }
    }

    /** Hand a command to Rust; the next frame applies it. */
    private static void sendMediaCommand(int command) {
        nativeMediaCommand(command);
    }

    /** Called from Rust when the transport starts or stops. */
    public static void setMediaPlaybackState(boolean isPlaying) {
        playing = isPlaying;
        publishPlaybackState();
        showNotification();
    }

    /** Called from Rust when the loaded dance or tune changes. */
    public static void setMediaMetadata(String dance, String tune, String albumName) {
        title = dance;
        artist = tune;
        album = albumName;
        publishMetadata();
        showNotification();
    }

    private static void publishMetadata() {
        if (mediaSession == null) {
            return;
        }
        MediaMetadata metadata = new MediaMetadata.Builder()
                .putString(MediaMetadata.METADATA_KEY_TITLE, title)
                .putString(MediaMetadata.METADATA_KEY_ARTIST, artist)
                .putString(MediaMetadata.METADATA_KEY_ALBUM, album)
                .build();
        mediaSession.setMetadata(metadata);
    }

    private static void publishPlaybackState() {
        if (mediaSession == null) {
            return;
        }
        PlaybackState state = new PlaybackState.Builder()
                .setActions(PlaybackState.ACTION_PLAY
                        | PlaybackState.ACTION_PAUSE
                        | PlaybackState.ACTION_PLAY_PAUSE
                        | PlaybackState.ACTION_STOP)
                .setState(
                        playing ? PlaybackState.STATE_PLAYING : PlaybackState.STATE_PAUSED,
                        PlaybackState.PLAYBACK_POSITION_UNKNOWN,
                        1.0f)
                .build();
        mediaSession.setPlaybackState(state);
    }

    /**
     * Show the MediaStyle notification, on Android 12 and older in particular.
     *
     * Android 13+ builds its own media card from the session, so the
     * notification here is a fallback that also gives the app a place in the
     * shade. It is not a foreground service: Android may still stop the app in
     * the background, and playback stops with it.
     */
    private static void showNotification() {
        if (mediaSession == null || appContext == null) {
            return;
        }
        ensureChannel();
        Notification.Action action = playing
                ? new Notification.Action.Builder(
                        android.R.drawable.ic_media_pause, "Pause", actionIntent(ACTION_PAUSE))
                        .build()
                : new Notification.Action.Builder(
                        android.R.drawable.ic_media_play, "Play", actionIntent(ACTION_PLAY))
                        .build();
        Notification.MediaStyle style = new Notification.MediaStyle()
                .setMediaSession(mediaSession.getSessionToken())
                .setShowActionsInCompactView(0);
        Notification.Builder builder = Build.VERSION.SDK_INT >= 26
                ? new Notification.Builder(appContext, CHANNEL_ID)
                : new Notification.Builder(appContext);
        builder.setSmallIcon(smallIcon())
                .setContentTitle(title)
                .setContentText(artist)
                .setContentIntent(contentIntent())
                .setStyle(style)
                .setVisibility(Notification.VISIBILITY_PUBLIC)
                .setOnlyAlertOnce(true)
                .setShowWhen(false)
                .setOngoing(playing)
                .addAction(action);
        try {
            NotificationManager manager =
                    (NotificationManager) appContext.getSystemService(Context.NOTIFICATION_SERVICE);
            if (manager != null) {
                manager.notify(NOTIFICATION_ID, builder.build());
            }
        } catch (SecurityException denied) {
            // POST_NOTIFICATIONS is denied: the notification stays hidden, and
            // the media session keeps receiving the same transport commands.
        }
    }

    private static void ensureChannel() {
        if (Build.VERSION.SDK_INT < 26) {
            return;
        }
        NotificationManager manager =
                (NotificationManager) appContext.getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager == null || manager.getNotificationChannel(CHANNEL_ID) != null) {
            return;
        }
        NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID, "Playback", NotificationManager.IMPORTANCE_LOW);
        channel.setDescription("Play and pause the dance session from outside the app");
        channel.setShowBadge(false);
        manager.createNotificationChannel(channel);
    }

    private static int smallIcon() {
        int icon = appContext.getApplicationInfo().icon;
        return icon != 0 ? icon : android.R.drawable.ic_media_play;
    }

    private static PendingIntent contentIntent() {
        Intent intent = new Intent(appContext, MainActivity.class);
        intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        return PendingIntent.getActivity(
                appContext, 0, intent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    private static PendingIntent actionIntent(String action) {
        Intent intent = new Intent(action).setPackage(appContext.getPackageName());
        return PendingIntent.getBroadcast(
                appContext, action.hashCode(), intent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
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

    /**
     * Open the system document picker for the tune or dance importer.
     *
     * <p>Used by Rust through JNI when the user chooses Import. The activity
     * result comes back in {@link #onActivityResult}, is read as UTF-8, and is
     * handed to Rust through {@link #nativeImportedFile}.
     */
    public static void openDocument(final int kind, final String mimeType, final boolean allowMultiple) {
        final MainActivity activity = appActivity;
        if (activity == null) {
            return;
        }
        importKind = kind;
        // The JNI call arrives on the native event-loop thread; starting an
        // activity must happen on the UI thread.
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType(mimeType);
                if (allowMultiple) {
                    intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                }
                activity.startActivityForResult(intent, REQUEST_IMPORT);
            }
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQUEST_IMPORT || resultCode != RESULT_OK || data == null) {
            return;
        }
        if (data.getClipData() != null) {
            int count = data.getClipData().getItemCount();
            for (int index = 0; index < count; index++) {
                deliverImport(data.getClipData().getItemAt(index).getUri());
            }
        } else if (data.getData() != null) {
            deliverImport(data.getData());
        }
    }

    private static void deliverImport(Uri uri) {
        if (uri == null || appContext == null) {
            return;
        }
        String name = queryDisplayName(uri);
        String text = readText(uri);
        if (text != null) {
            nativeImportedFile(importKind, name, text);
        }
    }

    private static String queryDisplayName(Uri uri) {
        try (Cursor cursor = appContext
                .getContentResolver()
                .query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (column >= 0) {
                    String name = cursor.getString(column);
                    if (name != null && !name.isEmpty()) {
                        return name;
                    }
                }
            }
        } catch (Exception ignored) {
            // Fall through to the path-based name.
        }
        String path = uri.getPath();
        return path != null ? path : "import";
    }

    private static String readText(Uri uri) {
        try (InputStream input = appContext.getContentResolver().openInputStream(uri)) {
            if (input == null) {
                return null;
            }
            ByteArrayOutputStream output = new ByteArrayOutputStream();
            byte[] buffer = new byte[8192];
            int read;
            while ((read = input.read(buffer)) != -1) {
                if (output.size() + read > MAX_IMPORT_BYTES) {
                    return null;
                }
                output.write(buffer, 0, read);
            }
            return new String(output.toByteArray(), StandardCharsets.UTF_8);
        } catch (Exception error) {
            return null;
        }
    }

    /**
     * Share an exported text file through Android's share sheet.
     *
     * <p>On Android 10 and newer the text is first copied into Downloads
     * through {@link MediaStore}, which returns a {@code content://} URI the
     * receiving app can read with the grant flag. A FileProvider would need a
     * provider declaration in the manifest, and cargo-apk does not support one;
     * the MediaStore URI avoids that without exposing a {@code file://} URI.
     * On Android 8 and 9, where the Downloads collection does not exist, the
     * share carries {@link Intent#EXTRA_TEXT} only.
     */
    public static void shareText(final String name, final String mimeType, final String text) {
        final MainActivity activity = appActivity;
        if (activity == null) {
            return;
        }
        // The JNI call arrives on the native event-loop thread; the chooser must
        // be started on the UI thread.
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Uri uri = writeToDownloads(name, mimeType, text);
                Intent send = new Intent(Intent.ACTION_SEND);
                send.setType(mimeType);
                send.putExtra(Intent.EXTRA_SUBJECT, name);
                send.putExtra(Intent.EXTRA_TEXT, text);
                if (uri != null) {
                    send.putExtra(Intent.EXTRA_STREAM, uri);
                    send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                }
                activity.startActivity(Intent.createChooser(send, "Share " + name));
            }
        });
    }

    /** Copy a text file into Downloads and return its content URI, or null. */
    private static Uri writeToDownloads(String name, String mimeType, String text) {
        if (Build.VERSION.SDK_INT < 29 || appContext == null) {
            return null;
        }
        ContentResolver resolver = appContext.getContentResolver();
        ContentValues values = new ContentValues();
        values.put(MediaStore.MediaColumns.DISPLAY_NAME, name);
        values.put(MediaStore.MediaColumns.MIME_TYPE, mimeType);
        values.put(
                MediaStore.MediaColumns.RELATIVE_PATH,
                Environment.DIRECTORY_DOWNLOADS + "/Caller Karaoke");
        values.put(MediaStore.MediaColumns.IS_PENDING, 1);
        Uri uri = null;
        try {
            uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values);
            if (uri == null) {
                return null;
            }
            try (OutputStream output = resolver.openOutputStream(uri)) {
                if (output == null) {
                    return null;
                }
                output.write(text.getBytes(StandardCharsets.UTF_8));
            }
            values.clear();
            values.put(MediaStore.MediaColumns.IS_PENDING, 0);
            resolver.update(uri, values, null, null);
            return uri;
        } catch (Exception error) {
            if (uri != null) {
                resolver.delete(uri, null, null);
            }
            return null;
        }
    }

    /**
     * Create the invisible field that hosts the system IME.
     *
     * <p>The content view of a native activity is a plain {@link View}, so the
     * IME refuses to show for it. This field is one pixel in the corner, fully
     * transparent, and never receives touches outside that pixel; it exists
     * only to own an {@link InputConnection}. The app shows and hides it from
     * Rust through {@link #setTextInputActive}.
     */
    private void createTextInput() {
        EditText input = new EditText(this) {
            @Override
            public boolean onKeyDown(int keyCode, KeyEvent event) {
                if (forwardKey(event)) {
                    return true;
                }
                return super.onKeyDown(keyCode, event);
            }

            @Override
            public boolean onKeyMultiple(int keyCode, int repeatCount, KeyEvent event) {
                String characters = event.getCharacters();
                if (characters != null && !characters.isEmpty()) {
                    for (int index = 0; index < repeatCount; index++) {
                        nativeImeText(characters);
                    }
                    return true;
                }
                return super.onKeyMultiple(keyCode, repeatCount, event);
            }

            @Override
            public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
                // No suggestions or autocorrect: aliases, searches, and ABC
                // source must reach the app exactly as typed. It also keeps
                // most IMEs committing text one character at a time, which
                // the bridge can forward without tracking candidate windows.
                outAttrs.inputType =
                        InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS;
                outAttrs.imeOptions = EditorInfo.IME_ACTION_DONE
                        | EditorInfo.IME_FLAG_NO_EXTRACT_UI
                        | EditorInfo.IME_FLAG_NO_FULLSCREEN;
                return new BridgeInputConnection(this);
            }
        };
        input.setSingleLine(true);
        input.setInputType(
                InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        input.setImeOptions(EditorInfo.IME_ACTION_DONE
                | EditorInfo.IME_FLAG_NO_EXTRACT_UI
                | EditorInfo.IME_FLAG_NO_FULLSCREEN);
        input.setBackground(null);
        input.setTextColor(0x00000000);
        input.setAlpha(0.0f);
        input.setCursorVisible(false);
        input.setFocusable(true);
        input.setFocusableInTouchMode(true);
        input.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        addContentView(input, new ViewGroup.LayoutParams(1, 1));
        textInput = input;
    }

    /**
     * Show or hide the system keyboard for the focused egui text field.
     *
     * <p>Called from Rust when the desired keyboard state changes (and again
     * on a tap, so a dismissed keyboard can be brought back). The call arrives
     * on the native event-loop thread and is posted to the UI thread.
     *
     * <p>This is the only Android IME controller: Rust suppresses egui's own
     * IME output so {@code winit} cannot issue a second, competing show/hide.
     * Each method call is therefore idempotent and cheap.
     */
    public static void setTextInputActive(final boolean active) {
        final MainActivity activity = appActivity;
        final EditText input = textInput;
        if (activity == null || input == null) {
            return;
        }
        activity.runOnUiThread(() -> {
            InputMethodManager imm =
                    input.getContext().getSystemService(InputMethodManager.class);
            if (active) {
                if (!input.hasFocus()) {
                    input.requestFocus();
                }
                if (imm != null && !imm.isActive(input)) {
                    input.post(() -> {
                        if (input.hasFocus() && !imm.isActive(input)) {
                            imm.showSoftInput(input, InputMethodManager.SHOW_IMPLICIT);
                        }
                    });
                }
            } else {
                if (imm != null) {
                    imm.hideSoftInputFromWindow(input.getWindowToken(), 0);
                }
                input.clearFocus();
                // The field only carries the IME. Clearing it after it loses
                // focus drops any half-composed text instead of leaking it
                // into the next field the user opens. Do not clear it while
                // showing: that restarts the IME (and its composing state) and
                // is what made numeric fields flicker.
                input.setText("");
            }
        });
    }

    /** Forward one key from the IME or a hardware keyboard to Rust. */
    private static boolean forwardKey(KeyEvent event) {
        if (event.getAction() == KeyEvent.ACTION_UP) {
            return handledKey(event.getKeyCode());
        }
        switch (event.getKeyCode()) {
            case KeyEvent.KEYCODE_DEL:
                nativeImeKey(KEY_BACKSPACE);
                return true;
            case KeyEvent.KEYCODE_FORWARD_DEL:
                nativeImeKey(KEY_DELETE);
                return true;
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
                nativeImeKey(KEY_ENTER);
                return true;
            case KeyEvent.KEYCODE_DPAD_LEFT:
                nativeImeKey(KEY_LEFT);
                return true;
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                nativeImeKey(KEY_RIGHT);
                return true;
            case KeyEvent.KEYCODE_DPAD_UP:
                nativeImeKey(KEY_UP);
                return true;
            case KeyEvent.KEYCODE_DPAD_DOWN:
                nativeImeKey(KEY_DOWN);
                return true;
            default:
                break;
        }
        if (event.isCtrlPressed() || event.isAltPressed()) {
            return false;
        }
        int unicode = event.getUnicodeChar();
        if (unicode != 0) {
            nativeImeText(new String(Character.toChars(unicode)));
            return true;
        }
        return false;
    }

    private static boolean handledKey(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_DEL:
            case KeyEvent.KEYCODE_FORWARD_DEL:
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
            case KeyEvent.KEYCODE_DPAD_LEFT:
            case KeyEvent.KEYCODE_DPAD_RIGHT:
            case KeyEvent.KEYCODE_DPAD_UP:
            case KeyEvent.KEYCODE_DPAD_DOWN:
                return true;
            default:
                return false;
        }
    }

    /** Send committed text, turning line breaks into Enter key presses. */
    private static void sendText(String text) {
        int start = 0;
        for (int index = 0; index < text.length(); index++) {
            char character = text.charAt(index);
            if (character == '\n' || character == '\r') {
                if (index > start) {
                    nativeImeText(text.substring(start, index));
                }
                nativeImeKey(KEY_ENTER);
                if (character == '\r'
                        && index + 1 < text.length()
                        && text.charAt(index + 1) == '\n') {
                    index++;
                }
                start = index + 1;
            }
        }
        if (start < text.length()) {
            nativeImeText(text.substring(start));
        }
    }

    /**
     * The input connection behind {@link #createTextInput}.
     *
     * <p>The field has no text of its own: every edit is forwarded to Rust and
     * egui applies it to the focused text field. That keeps the real text,
     * selection, undo, and validation in one place.
     */
    private static final class BridgeInputConnection extends BaseInputConnection {
        /** The text the IME is currently composing, already shown in the app. */
        private String composing = "";

        BridgeInputConnection(EditText target) {
            super(target, false);
        }

        @Override
        public boolean commitText(CharSequence text, int newCursorPosition) {
            String value = text.toString();
            if (!composing.isEmpty()) {
                if (value.equals(composing)) {
                    // Every composing character already reached the app.
                    composing = "";
                    return true;
                }
                deleteComposing();
            }
            sendText(value);
            return true;
        }

        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            String value = text.toString();
            if (value.startsWith(composing)) {
                // An IME adds one character at a time; forward only the suffix.
                sendText(value.substring(composing.length()));
            } else {
                deleteComposing();
                sendText(value);
            }
            composing = value;
            return true;
        }

        @Override
        public boolean finishComposingText() {
            composing = "";
            return true;
        }

        @Override
        public boolean setComposingRegion(int start, int end) {
            return true;
        }

        @Override
        public boolean deleteSurroundingTextInCodePoints(int beforeLength, int afterLength) {
            return deleteSurroundingText(beforeLength, afterLength);
        }

        @Override
        public CharSequence getTextBeforeCursor(int length, int flags) {
            // The real text lives in egui. An empty string keeps IMEs that ask
            // for the surrounding text from receiving a null.
            return "";
        }

        @Override
        public CharSequence getTextAfterCursor(int length, int flags) {
            return "";
        }

        @Override
        public CharSequence getSelectedText(int flags) {
            return null;
        }

        @Override
        public int getCursorCapsMode(int reqModes) {
            return 0;
        }

        @Override
        public boolean deleteSurroundingText(int beforeLength, int afterLength) {
            composing = "";
            for (int index = 0; index < beforeLength; index++) {
                nativeImeKey(KEY_BACKSPACE);
            }
            for (int index = 0; index < afterLength; index++) {
                nativeImeKey(KEY_DELETE);
            }
            return true;
        }

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            forwardKey(event);
            return true;
        }

        @Override
        public boolean performEditorAction(int actionCode) {
            nativeImeKey(KEY_ENTER);
            return true;
        }

        /** Remove the composing text already applied in the app. */
        private void deleteComposing() {
            int count = composing.codePointCount(0, composing.length());
            for (int index = 0; index < count; index++) {
                nativeImeKey(KEY_BACKSPACE);
            }
            composing = "";
        }
    }

    /** Publish this activity's class to the media bridge; see `media_keys.rs`. */
    private static native void nativeAttachActivity();

    private static native void nativeSetSystemBarInsets(
            int left, int top, int right, int bottom);

    /** Transport command from the media session; see `media_keys.rs`. */
    private static native void nativeMediaCommand(int command);

    /** A file chosen in the document picker; see `files.rs`. */
    private static native void nativeImportedFile(int kind, String name, String text);

    /** Text committed by the system IME; see `android_ime.rs`. */
    private static native void nativeImeText(String text);

    /** One editing key from the system IME; see `android_ime.rs`. */
    private static native void nativeImeKey(int key);
}
