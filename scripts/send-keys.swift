// vkTERM — envía pulsaciones sintetizadas (CGEvent) a un proceso concreto.
//
//   send-keys <pid> <combo>...
//   combo: [cmd+][lopt+|ropt+][ctrl+][shift+]<tecla>  ·  text:<ascii>  ·  sleep:<ms>
//   teclas: a-z, 0-9, left, right, up, down, enter, esc, tab, space, backspace,
//           comma, period
//
// Lo usa `scripts/smoke-test.py --ghostty`. Los eventos se entregan solo al
// proceso <pid> (CGEvent.postToPid): no roban el foco ni pueden llegar a otra
// aplicación. Desde ahí recorren el mismo camino que el teclado (Ghostty →
// Zellij → aplicación), pero NO son pulsaciones físicas: no pasan por los
// atajos globales del sistema. Requiere permiso de Accesibilidad para el
// terminal que lo ejecuta.
import AppKit
import CoreGraphics

let keyCodes: [String: CGKeyCode] = [
    "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9,
    "b": 11, "q": 12, "w": 13, "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19,
    "3": 20, "4": 21, "6": 22, "5": 23, "9": 25, "7": 26, "8": 28, "0": 29, "o": 31,
    "u": 32, "i": 34, "p": 35, "l": 37, "j": 38, "k": 40, "n": 45, "m": 46,
    "comma": 43, "period": 47,
    "enter": 36, "tab": 48, "space": 49, "backspace": 51, "esc": 53,
    "left": 123, "right": 124, "down": 125, "up": 126,
]

// Máscara general + bit de dispositivo (izquierda/derecha), como el teclado real.
let modifierFlags: [String: UInt64] = [
    "cmd": CGEventFlags.maskCommand.rawValue | 0x08,
    "lopt": CGEventFlags.maskAlternate.rawValue | 0x20,
    "ropt": CGEventFlags.maskAlternate.rawValue | 0x40,
    "ctrl": CGEventFlags.maskControl.rawValue | 0x01,
    "shift": CGEventFlags.maskShift.rawValue | 0x02,
]

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(("send-keys: " + message + "\n").data(using: .utf8)!)
    exit(1)
}

func press(_ code: CGKeyCode, flags: UInt64, pid: pid_t) {
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else {
            fail("no se pudo crear el evento")
        }
        event.flags = CGEventFlags(rawValue: flags)
        event.postToPid(pid)
        usleep(15_000)
    }
}

let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]) else {
    fail("uso: send-keys <pid> <combo>...")
}
guard NSRunningApplication(processIdentifier: pid) != nil else {
    fail("no existe el proceso \(pid)")
}

for combo in args.dropFirst(2) {
    if combo.hasPrefix("sleep:") {
        usleep(UInt32(combo.dropFirst(6))! * 1000)
    } else if combo.hasPrefix("text:") {
        for character in combo.dropFirst(5) {
            let key = String(character).lowercased()
            guard let code = keyCodes[key] else { fail("carácter no soportado: \(character)") }
            press(code, flags: character.isUppercase ? modifierFlags["shift"]! : 0, pid: pid)
        }
    } else {
        var parts = combo.split(separator: "+").map(String.init)
        let key = parts.removeLast()
        guard let code = keyCodes[key] else { fail("tecla no soportada: \(key)") }
        var flags: UInt64 = 0
        for modifier in parts {
            guard let flag = modifierFlags[modifier] else { fail("modificador no soportado: \(modifier)") }
            flags |= flag
        }
        press(code, flags: flags, pid: pid)
    }
    usleep(120_000)
}
