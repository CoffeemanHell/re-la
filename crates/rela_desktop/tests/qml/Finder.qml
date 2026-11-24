// Visible items under root whose `text` equals t (or matches a predicate).
pragma Singleton
import QtQuick

QtObject {
    function all(root, pred) {
        let out = []
        if (!root) return out
        if (pred(root)) out.push(root)
        const kids = root.children || []
        for (let i = 0; i < kids.length; i++) out = out.concat(all(kids[i], pred))
        if (root.contentItem && kids.indexOf(root.contentItem) < 0) out = out.concat(all(root.contentItem, pred))
        return out
    }
    function text(root, t) { return all(root, it => it.visible && it.text === t)[0] || null }
}
