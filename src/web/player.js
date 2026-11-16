// Runs inside Audible's own web player page. Skald never touches the audio
// stream: it reads what the page shows and presses the page's own controls,
// so playback, licensing, and position sync all stay Audible's.
(() => {
    if (window.__skald)
        return;

    const audio = () => document.getElementById('audible-player') || document.querySelector('audio');
    const byTest = id => document.querySelector(`[data-testid="${id}"]`);
    const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

    // "1:02:03" or "17:48" to seconds.
    const clock = text => String(text || '').trim().split(':').reduce((sum, part) => sum * 60 + (parseInt(part, 10) || 0), 0);

    const chapters = () => {
        const list = [];
        for (let i = 0; ; i++) {
            const title = byTest(`title-section-${i}`);
            if (!title)
                break;
            const duration = byTest(`duration-section-${i}`);
            list.push({ title: title.textContent.trim(), duration: clock(duration && duration.textContent) });
        }
        return list;
    };

    // The page forgets a speed it did not set itself when it loads the next
    // part, so the wanted speed is put back whenever it has drifted.
    let wantRate = null;
    let sentChapters = '';

    window.__skald = {
        state() {
            const a = audio();
            const meta = navigator.mediaSession && navigator.mediaSession.metadata;
            const seek = document.querySelector('input[type="range"]');
            if (a && wantRate && Math.abs(a.playbackRate - wantRate) > 0.001)
                a.playbackRate = wantRate;

            const state = {
                path: location.pathname,
                ready: !!a && a.readyState >= 1 && isFinite(a.duration) && a.duration > 0,
                playing: !!a && !a.paused && !a.ended,
                position: a ? a.currentTime : 0,
                duration: a && isFinite(a.duration) ? a.duration : 0,
                rate: a ? a.playbackRate : 1,
                title: meta ? meta.title : '',
                chapter: meta ? meta.artist : '',
                cover: meta && meta.artwork && meta.artwork[0] ? meta.artwork[0].src : '',
                chapterPosition: seek ? parseFloat(seek.value) || 0 : 0,
                chapterDuration: seek ? parseFloat(seek.max) || 0 : 0,
                error: a && a.error ? (a.error.message || `media error ${a.error.code}`) : '',
            };

            // The chapter list is long and rarely changes; send it only then.
            const list = chapters();
            const signature = list.length + '|' + (list[0] ? list[0].title : '') + '|' + (list.length ? list[list.length - 1].title : '');
            if (signature !== sentChapters) {
                sentChapters = signature;
                state.chapters = list;
            }
            return JSON.stringify(state);
        },

        command(name, value) {
            const a = audio();
            if (!a)
                return;
            const press = id => { const button = byTest(id); if (button) button.click(); return !!button; };
            switch (name) {
            case 'play':
                if (a.paused && !press('play')) a.play();
                break;
            case 'pause':
                if (!a.paused && !press('pause')) a.pause();
                break;
            case 'toggle':
                if (a.paused) { if (!press('play')) a.play(); }
                else if (!press('pause')) a.pause();
                break;
            case 'skip':
                a.currentTime = clamp(a.currentTime + value, 0, a.duration || 0);
                break;
            case 'seekChapter': {
                // The page's slider is relative to the chapter; the audio is the whole book.
                const seek = document.querySelector('input[type="range"]');
                const start = a.currentTime - (seek ? parseFloat(seek.value) || 0 : 0);
                const length = seek ? parseFloat(seek.max) || 0 : 0;
                a.currentTime = clamp(start + clamp(value, 0, Math.max(0, length - 1)), 0, a.duration || 0);
                break;
            }
            case 'previousChapter':
                press('previous-section');
                break;
            case 'nextChapter':
                press('next-section');
                break;
            case 'chapter': {
                const title = byTest(`title-section-${value}`);
                const button = title && title.closest('button');
                if (button) button.click();
                break;
            }
            case 'rate':
                wantRate = value;
                a.playbackRate = value;
                break;
            }
        },
    };
})();
