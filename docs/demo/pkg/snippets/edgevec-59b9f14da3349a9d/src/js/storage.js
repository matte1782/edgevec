export class IndexedDbBackend {
    static async write(name, data) {
        return new Promise((resolve, reject) => {
            const req = indexedDB.open("EdgeVecDB", 1);
            req.onupgradeneeded = (e) => {
                const db = e.target.result;
                if (!db.objectStoreNames.contains('files')) {
                    db.createObjectStore('files');
                }
            };
            req.onsuccess = (e) => {
                const db = e.target.result;
                const tx = db.transaction(['files'], 'readwrite');
                const store = tx.objectStore('files');
                store.put(data, name);
                tx.oncomplete = () => {
                    resolve();
                };
                tx.onerror = () => reject(tx.error);
            };
            req.onerror = () => reject(req.error);
        });
    }

    static async read(name) {
        return new Promise((resolve, reject) => {
            const req = indexedDB.open("EdgeVecDB", 1);
            req.onsuccess = (e) => {
                const db = e.target.result;
                const tx = db.transaction(['files'], 'readonly');
                const store = tx.objectStore('files');
                const getReq = store.get(name);
                getReq.onsuccess = () => {
                    if (getReq.result) {
                        resolve(getReq.result);
                    } else {
                        reject(new Error(`File ${name} not found`));
                    }
                };
                getReq.onerror = () => reject(getReq.error);
            };
            req.onerror = () => reject(req.error);
        });
    }
}

