# Using re-la

The interface is in Turkish; button names are given here as you see them.

## Sign in

Enter your Navidrome address (e.g. `https://music.example.com`), user name and password. The password is not saved; re-la keeps a token and signs in automatically next time.

## Listening together

Click the button at the top right that says who is online (**… kişi çevrimiçi**, or **Yalnız** when nobody is) and press **Dinle** (listen) next to someone. You hear what they play, at the same second. **Ayrıl** (leave) goes back to playing on your own.

This needs the relay on the server ([relay/README.md](../relay/README.md)). Without it, re-la is a normal player.

## The DJ

With **DJ modu** on (Settings → DJ; on the desktop also the **DJ** button in the player bar, on Android the ⋯ menu on Now Playing), re-la keeps the queue going with songs it picks itself. It learns on this device from what you play to the end and what you skip early, and keeps no data on the server.

- **Why this song?** Each song the DJ adds says why under its name, in the queue and on Now Playing: "close to the shoegaze you just heard", "not played for 3 months", "new in the library", "surprise".
- **More like this / Less like this:** in a song's menu (desktop) or the ⋯ menu on Now Playing. Clearer than a skip. *Less like this* also moves on to the next song without counting a skip. Settings → DJ → **Geri bildirimlerin** lists every choice; **Kaldır** takes one back.
- **Undo a skip:** an early skip counts against a song after a short wait. Until then a notice says **Atlama eksi sayılacak** with **Geri al** (undo; on the desktop it sits in the bar at the bottom), for the times you skipped because of the moment, not the song.
- **DJ durumu** (DJ state, in Settings → DJ): the songs the DJ weighed for its last pick, best first, with each score's parts: fit with what you just played, long-term taste, Navidrome's "similar", new or rarely played, played lately, your feedback. The picked ones stand out.
- **Settings:** every rule the DJ follows can be changed, for example how early a skip counts against a song, after how many such skips (on how many different days) a song counts as disliked, how long it remembers and how many songs per artist. The math lives under **Gelişmiş ayarlar** (advanced). **Varsayılanlara dön** puts them all back. **DJ hafızasını sıfırla** forgets skips, listens and feedback; the library index stays.

## SUB/WAVE radio

If a radio station in Navidrome is a [SUB/WAVE](https://www.getsubwave.com) station, re-la shows what is on air: the real track and cover, the DJ and show, lyrics, the upcoming tracks, a heart button and a request button.

Add the station in Navidrome (Radio → add). The stream URL is enough: re-la asks the stream's own address (`https://radio.example.com/api/now-playing`) whether it is SUB/WAVE. If the SUB/WAVE site lives elsewhere, put its address in **Home page URL** and that is asked instead.

- **Stream URL:** `https://radio.example.com/stream.flac?auth=PASSWORD` (the `?auth=` part only for a private station; `https://user:PASSWORD@radio.example.com/stream.mp3` works too)
- **Home page URL:** optional

The password is sent only to the lyrics endpoint, as a header. Without it, lyrics say **Parola gerekli** (password needed). Other stations work as before.

While a SUB/WAVE station plays, the Now Playing panel has **Sözler** (lyrics) and **İstek** (request a song, with the upcoming tracks below), and the bar at the bottom shows how far into the track the station is. The bar follows what you hear when the station reports its stream buffer (`bufferSeconds`); a station that does not can run a few seconds ahead.

In the **listen together** window, someone on a radio station shows as `📻 Station · Title · Artist`. This needs the updated relay on the server; with an older relay the list shows the song the person's queue is paused on.

The line at the bottom of Now Playing shows what the stream is, as mpv decodes it, for any radio station: codec, sample rate, channels and bitrate (for example `FLAC · 44,1 kHz · stereo · 890 kbps`). Parts mpv does not know yet are left out.

While a SUB/WAVE station plays, re-la also tells Navidrome what is on air for **your** account, like it does for songs: it shows as now playing (so a Discord presence plugin on the server picks it up) and the play counts once half of the track (at most 4 minutes) was heard. SUB/WAVE has its own Navidrome user, so the counts do not add up twice. A track you tuned in to half way through is shown but not counted. The settings that switch reporting and scrobbling off for songs switch it off here too.

Hearts and requests go out from your own connection, so they are counted for you, not for the person you listen with.

## Without the server

With a size limit set (settings, **Önbellek**; on Android **Veri ve depolama**) re-la keeps songs on disk: the next ones in the queue (**Önden inen** says how many) and each song that counted as played. When the limit is reached, the one not heard for the longest time goes. If the server can't be reached at start, re-la opens with the last queue and plays the songs it has; it keeps trying to connect. Plays made meanwhile are not counted on the server. A song you jump to yourself is loaded twice, once for playing and once for the disk.

## Transcoding (Android)

**Veri ve depolama** has a choice for Wi-Fi and for mobile data. With re-with (and its Navidrome account, see `relay/README.md`) it lists Navidrome's own transcoding profiles with their bitrates. Without it, it lists the formats the server says it can make (OpenSubsonic `transcoding`, recent Navidrome versions), with a slider for the bitrate; Navidrome also counts its built-in formats there, even when their profile was deleted. A server that can't tell leaves only **Orijinal**. A change applies from the next song on, and a song that is already at or under the bitrate comes as it is.

## Shortcuts

| Key | Action |
|---|---|
| Space | Play / pause |
| Ctrl+→ / Ctrl+← | Next / previous |
| Ctrl+F | Search |
| Alt+← / Alt+→ | Back / forward |

They can be changed in the settings (**Klavye**).

Right-click a song, album, artist or playlist for more actions.

## Troubleshooting

- **Cannot connect:** open the address in a browser and check that Navidrome loads.
- **No button for who is online:** the relay is not installed or not reachable.
- **Closing the window quits:** switch on **Sistem tepsisi** in the settings to keep playing in the tray. Right-click the tray icon → **Çıkış** quits then.
- **Sound from the wrong device:** pick it in the settings, **Çalma** → **Ses çıkışı**.
