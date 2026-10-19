// IndexedDB backend for EdgeVec.save() / EdgeVec.load().
//
// Every IndexedDB callback runs outside the Promise executor, so each one is
// wrapped: a synchronous throw (e.g. a missing object store) must reject the
// promise instead of leaving it pending forever (issue #12).

const DB_NAME = "EdgeVecDB";
const STORE_NAME = "files";

// Reject callers while an upgrade is blocked. Rejecting its Promise cannot
// cancel the IDB request: another open would queue behind it indefinitely.
let blockedOpen = null;
// Serialize connection setup (not transactions). Otherwise concurrent callers
// can queue their own repair requests before the first request fires blocked.
const openQueues = new WeakMap();

// Opens the database at its current version (creating it with the store if
// absent). A database can exist WITHOUT the store: EdgeVec <= 0.9.0 opened it
// from read() with no upgrade handler, leaving an empty v1 database. Only in
// that case is the version bumped to run the upgrade and repair it, so healthy
// databases never trigger a version change (which other open tabs could block).
async function openDb() {
    if (typeof indexedDB === "undefined") {
        throw new Error(
            "IndexedDB is not available in this environment. " +
            "Use toBytes()/EdgeVec.fromBytes() to persist the index yourself."
        );
    }
    const factory = indexedDB;
    const previous = openQueues.get(factory) ?? Promise.resolve();
    const opening = previous.then(() => openDbAt(factory));
    // Keep the queue moving after failures; each caller still gets its error.
    openQueues.set(factory, opening.then(() => undefined, () => undefined));
    return opening;
}

async function openDbAt(factory) {
    if (blockedOpen?.factory === factory) {
        throw blockedOpen.error;
    }
    let db = await openAt(factory, undefined);
    for (let attempt = 0; !db.objectStoreNames.contains(STORE_NAME); attempt++) {
        const next = db.version + 1;
        db.close();
        if (attempt === 2) {
            throw new Error(`${DB_NAME} is missing the "${STORE_NAME}" object store`);
        }
        try {
            db = await openAt(factory, next);
        } catch (e) {
            // A concurrent repair already moved past `next`; reopen as-is.
            if (e && e.name === "VersionError") {
                db = await openAt(factory, undefined);
            } else {
                throw e;
            }
        }
    }
    return db;
}

function openAt(factory, version) {
    if (blockedOpen?.factory === factory) {
        return Promise.reject(blockedOpen.error);
    }
    return new Promise((resolve, reject) => {
        let settled = false;
        let req;
        try {
            req = version === undefined
                ? factory.open(DB_NAME)
                : factory.open(DB_NAME, version);
        } catch (e) {
            reject(e);
            return;
        }

        req.onupgradeneeded = () => {
            try {
                const db = req.result;
                if (!db.objectStoreNames.contains(STORE_NAME)) {
                    db.createObjectStore(STORE_NAME);
                }
            } catch (e) {
                // Aborting the upgrade transaction fires req.onerror.
                try { req.transaction.abort(); } catch (_) { /* already aborted */ }
                settled = true;
                reject(e);
            }
        };
        req.onsuccess = () => {
            if (blockedOpen?.request === req) blockedOpen = null;
            const db = req.result;
            if (settled) {
                db.close();
                return;
            }
            settled = true;
            // Never block a future version upgrade from another tab.
            db.onversionchange = () => db.close();
            resolve(db);
        };
        req.onerror = () => {
            if (blockedOpen?.request === req) blockedOpen = null;
            if (!settled) {
                settled = true;
                reject(req.error);
            }
        };
        req.onblocked = () => {
            if (!settled) {
                settled = true;
                const error = new Error(
                    `${DB_NAME} upgrade is blocked by another open connection ` +
                    "(close other tabs using EdgeVec and retry)"
                );
                blockedOpen = { request: req, error, factory };
                reject(error);
            }
        };
    });
}

// Runs `fn(store)` in a transaction; resolves with fn's result once the
// transaction commits, rejects on any error or abort. Always closes the db.
async function withStore(mode, fn) {
    const db = await openDb();
    return new Promise((resolve, reject) => {
        let result;
        try {
            const tx = db.transaction([STORE_NAME], mode);
            tx.oncomplete = () => {
                db.close();
                resolve(result);
            };
            tx.onerror = () => {
                db.close();
                reject(tx.error);
            };
            tx.onabort = () => {
                db.close();
                reject(tx.error || new Error("IndexedDB transaction aborted"));
            };
            const req = fn(tx.objectStore(STORE_NAME));
            req.onsuccess = () => {
                result = req.result;
            };
            // req errors propagate to tx.onerror; no separate handler needed.
        } catch (e) {
            db.close();
            reject(e);
        }
    });
}

export class IndexedDbBackend {
    static async write(name, data) {
        await withStore("readwrite", (store) => store.put(data, name));
    }

    static async read(name) {
        const value = await withStore("readonly", (store) => store.get(name));
        if (value === undefined) {
            const err = new Error(`No EdgeVec database saved under "${name}"`);
            err.name = "NotFoundError";
            throw err;
        }
        return value;
    }
}
