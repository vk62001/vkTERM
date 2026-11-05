// vkTERM — portapapeles para las pruebas, sin perder el del usuario.
//
//   pasteboard     (órdenes por la entrada estándar, una por línea)
//     count           el changeCount actual
//     set TEXTO       pone TEXTO (y devuelve el changeCount)
//     has TEXTO       "yes" si el texto del portapapeles contiene TEXTO, "no" si no
//     types           los tipos del primer elemento (nunca el contenido)
//     restore N       si el changeCount sigue siendo N, devuelve lo que había
//                     al arrancar; si no (el usuario copió algo), no lo toca
//
// Lo usa scripts/e2e-ghostty.py. Guarda el portapapeles del usuario solo en
// memoria (nunca en disco) y nunca escribe su contenido: puede tener
// contraseñas.
import AppKit

let board = NSPasteboard.general
let saved: [[(NSPasteboard.PasteboardType, Data)]] = (board.pasteboardItems ?? []).map { item in
    item.types.compactMap { type in item.data(forType: type).map { (type, $0) } }
}

func reply(_ text: String) {
    FileHandle.standardOutput.write((text + "\n").data(using: .utf8)!)
}

while let line = readLine() {
    let (command, argument) = line.firstIndex(of: " ").map {
        (String(line[..<$0]), String(line[line.index(after: $0)...]))
    } ?? (line, "")
    switch command {
    case "count":
        reply(String(board.changeCount))
    case "set":
        board.clearContents()
        board.setString(argument, forType: .string)
        reply(String(board.changeCount))
    case "has":
        reply((board.string(forType: .string) ?? "").contains(argument) ? "yes" : "no")
    case "types":
        reply((board.pasteboardItems?.first?.types ?? []).map(\.rawValue).joined(separator: ","))
    case "restore":
        guard Int(argument) == board.changeCount else {
            reply("kept")
            continue
        }
        board.clearContents()
        let items = saved.map { pairs -> NSPasteboardItem in
            let item = NSPasteboardItem()
            for (type, data) in pairs { item.setData(data, forType: type) }
            return item
        }
        board.writeObjects(items)
        reply("restored")
    default:
        reply("?")
    }
}
