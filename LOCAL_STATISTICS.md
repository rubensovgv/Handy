# Local statistics

Statistics uses a separate `dictation_statistics` table in `history.db` in
Handy's application data directory. It stores one counter per history ID
(day, word count and optional audio duration), without copying text or audio.
History cleanup never deletes these counters. New history writes and their
counters commit in one transaction; retrying updates the same counter.
Existing retained history is imported once per entry. Previously deleted
history cannot be recovered. Historical dates use the current local timezone;
new entries retain their local date at capture time.

Words are whitespace-separated tokens containing a letter or number in the
original transcription. WPM is total measured words / total available WAV
minutes, including recorded pauses. Entries without audio still count towards
words and streaks, but not WPM. A current streak remains active through the day
after the most recent dictation. Monthly comparison is month-to-date versus
the complete previous month, explicitly labelled in the UI.
