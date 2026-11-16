// Runs inside a signed-in audible page. Reads the same library pages the
// site shows, one after another, and hands the titles back as plain data.
(() => {
    if (window.__skaldLibrary)
        return;

    const text = element => element ? element.textContent.replace(/\s+/g, ' ').trim() : '';
    const names = (row, selector) => [...row.querySelectorAll(selector)].map(text).filter(Boolean);

    const parse = doc => [...doc.querySelectorAll('[id^="adbl-library-content-row-"]')].map(row => {
        const asin = row.id.replace('adbl-library-content-row-', '');
        const image = row.querySelector('img');
        const meter = row.querySelector('[role="progressbar"]');
        const finished = row.querySelector(`#time-remaining-finished-${CSS.escape(asin)}`);
        return {
            asin,
            title: text(row.querySelector('.bc-size-headline3')),
            authors: names(row, '.authorLabel a'),
            narrators: names(row, '.narratorLabel a'),
            series: names(row, '.seriesLabel a')[0] || '',
            cover: image ? image.getAttribute('src') || '' : '',
            percent: meter ? parseInt(meter.getAttribute('aria-valuenow'), 10) || 0 : 0,
            remaining: text(row.querySelector(`#time-remaining-display-${CSS.escape(asin)}`)),
            finished: !!finished && !finished.className.includes('bc-pub-hidden'),
        };
    }).filter(book => book.asin && book.title);

    const lastPage = doc => Math.max(1, ...[...doc.querySelectorAll('.pagingElements .pageNumberElement')]
        .map(element => parseInt(element.textContent, 10) || 0));

    const fresh = status => ({ status, loaded: 0, pages: 0, books: [], error: '' });

    window.__skaldLibrary = {
        progress: fresh('idle'),

        start() {
            if (this.progress.status === 'loading')
                return;
            this.progress = fresh('loading');
            this.run().catch(error => {
                this.progress.status = 'error';
                this.progress.error = String(error && error.message || error);
            });
        },

        async run() {
            const books = [];
            let pages = 1;
            // The cap only stops a runaway loop if the paging markup changes.
            for (let page = 1; page <= pages && page <= 60; page++) {
                const response = await fetch(`/library/titles?pageSize=50&page=${page}`, { credentials: 'include' });
                if (!new URL(response.url).pathname.startsWith('/library')) {
                    this.progress.status = 'signin';
                    return;
                }
                if (!response.ok)
                    throw new Error(`HTTP ${response.status}`);
                const doc = new DOMParser().parseFromString(await response.text(), 'text/html');
                const found = parse(doc);
                if (page === 1 && !found.length && !doc.querySelector('#adbl-library-content-main, .adbl-library-content, [id*="library"]'))
                    throw new Error('unrecognized library page');
                books.push(...found);
                pages = Math.max(pages, lastPage(doc));
                this.progress.loaded = books.length;
                this.progress.pages = pages;
            }
            this.progress.books = books;
            this.progress.status = 'done';
        },

        // The books travel only once, with the final state.
        state() {
            const p = this.progress;
            if (p.status === 'done') {
                this.progress = fresh('idle');
                return JSON.stringify(p);
            }
            return JSON.stringify({ status: p.status, loaded: p.loaded, pages: p.pages, error: p.error });
        },
    };
})();
