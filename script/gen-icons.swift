#!/usr/bin/env swift
import AppKit
import Foundation

let projectDir: URL
if CommandLine.arguments.count > 1 {
    projectDir = URL(fileURLWithPath: CommandLine.arguments[1])
} else {
    projectDir = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
}
let assetsDir = projectDir.appendingPathComponent("assets")

func drawAineerIcon(size: Int) -> Data? {
    let sz = CGFloat(size)
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .calibratedRGB, bytesPerRow: 0, bitsPerPixel: 0
    )!
    rep.size = NSSize(width: sz, height: sz)

    guard let ctx = NSGraphicsContext(bitmapImageRep: rep) else { return nil }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = ctx
    let gc = ctx.cgContext

    let u = sz / 512.0

    // Background: rounded rect with gradient
    let cornerRadius = 108.0 * u
    let bgPath = CGPath(roundedRect: CGRect(x: 0, y: 0, width: sz, height: sz),
                        cornerWidth: cornerRadius, cornerHeight: cornerRadius, transform: nil)
    gc.addPath(bgPath)
    gc.clip()

    let bgColors = [
        CGColor(red: 0.075, green: 0.075, blue: 0.168, alpha: 1.0),
        CGColor(red: 0.051, green: 0.051, blue: 0.122, alpha: 1.0),
    ]
    if let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                             colors: bgColors as CFArray, locations: [0.0, 1.0]) {
        gc.drawLinearGradient(grad, start: .zero, end: CGPoint(x: sz, y: sz), options: [])
    }

    // Border
    gc.resetClip()
    let borderPath = CGPath(roundedRect: CGRect(x: 1.5*u, y: 1.5*u, width: sz - 3*u, height: sz - 3*u),
                            cornerWidth: cornerRadius - 1.5*u, cornerHeight: cornerRadius - 1.5*u, transform: nil)
    gc.addPath(borderPath)
    gc.setStrokeColor(CGColor(red: 0.388, green: 0.4, blue: 0.945, alpha: 0.12))
    gc.setLineWidth(1.5 * u)
    gc.strokePath()

    // "A" shape — flipped coordinate system (CoreGraphics is bottom-left origin)
    let aPath = CGMutablePath()
    // Outer triangle (top-center to bottom-left to bottom-right)
    // SVG coords (top-left origin): M 256 72 L 440 432 L 72 432 Z
    // CG coords (bottom-left origin): flip Y -> y' = sz - y
    let topX = 256.0 * u, topY = sz - 72.0 * u
    let blX = 72.0 * u, blY = sz - 432.0 * u
    let brX = 440.0 * u, brY = sz - 432.0 * u
    // Left notch: 154 432
    let nlX = 154.0 * u, nlY = sz - 432.0 * u
    // Right notch: 358 432
    let nrX = 358.0 * u, nrY = sz - 432.0 * u
    // Left shoulder: 202 320
    let slX = 202.0 * u, slY = sz - 320.0 * u
    // Right shoulder: 310 320
    let srX = 310.0 * u, srY = sz - 320.0 * u

    // Outer A shape (same as SVG but with the cross-cut)
    aPath.move(to: CGPoint(x: topX, y: topY))
    aPath.addLine(to: CGPoint(x: brX, y: brY))
    aPath.addLine(to: CGPoint(x: nrX, y: nrY))
    aPath.addLine(to: CGPoint(x: srX, y: srY))
    aPath.addLine(to: CGPoint(x: slX, y: slY))
    aPath.addLine(to: CGPoint(x: nlX, y: nlY))
    aPath.addLine(to: CGPoint(x: blX, y: blY))
    aPath.closeSubpath()

    // Inner cutout triangle
    // SVG: M 256 186 L 216 292 L 296 292 Z
    let ciTopX = 256.0 * u, ciTopY = sz - 186.0 * u
    let ciBlX = 216.0 * u, ciBlY = sz - 292.0 * u
    let ciBrX = 296.0 * u, ciBrY = sz - 292.0 * u
    aPath.move(to: CGPoint(x: ciTopX, y: ciTopY))
    aPath.addLine(to: CGPoint(x: ciBlX, y: ciBlY))
    aPath.addLine(to: CGPoint(x: ciBrX, y: ciBrY))
    aPath.closeSubpath()

    // A gradient fill
    gc.saveGState()
    gc.addPath(aPath)
    gc.clip(using: .evenOdd)
    let aColors = [
        CGColor(red: 0.506, green: 0.549, blue: 0.973, alpha: 1.0),  // #818CF8
        CGColor(red: 0.388, green: 0.4, blue: 0.945, alpha: 1.0),    // #6366F1
        CGColor(red: 0.024, green: 0.714, blue: 0.831, alpha: 1.0),  // #06B6D4
    ]
    if let aGrad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                              colors: aColors as CFArray, locations: [0.0, 0.45, 1.0]) {
        // SVG gradient: x1=128 y1=64 x2=384 y2=448
        gc.drawLinearGradient(aGrad,
                              start: CGPoint(x: 128.0 * u, y: sz - 64.0 * u),
                              end: CGPoint(x: 384.0 * u, y: sz - 448.0 * u),
                              options: [])
    }
    gc.restoreGState()

    // Glow effect: draw the A shape again with slight blur (simplified glow)
    gc.saveGState()
    gc.setShadow(offset: .zero, blur: 8.0 * u,
                 color: CGColor(red: 0.388, green: 0.4, blue: 0.945, alpha: 0.4))
    gc.addPath(aPath)
    gc.clip(using: .evenOdd)
    if let aGrad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                              colors: aColors as CFArray, locations: [0.0, 0.45, 1.0]) {
        gc.drawLinearGradient(aGrad,
                              start: CGPoint(x: 128.0 * u, y: sz - 64.0 * u),
                              end: CGPoint(x: 384.0 * u, y: sz - 448.0 * u),
                              options: [])
    }
    gc.restoreGState()

    // AI spark dot (golden)
    let dotR = 9.0 * u
    let dotX = 330.0 * u
    let dotY = sz - 302.0 * u
    gc.setFillColor(CGColor(red: 0.984, green: 0.749, blue: 0.141, alpha: 0.85))
    gc.fillEllipse(in: CGRect(x: dotX - dotR, y: dotY - dotR, width: dotR * 2, height: dotR * 2))

    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [.interlaced: false])
}

print("Generating Aineer icons...")

// Generate standalone PNGs
for sz in [48, 256, 512, 1024] {
    if let png = drawAineerIcon(size: sz) {
        let path = assetsDir.appendingPathComponent("icon-\(sz).png")
        try! png.write(to: path)
        print("  ✓ icon-\(sz).png")
    }
}

// Generate macOS iconset
let iconsetDir = assetsDir.appendingPathComponent("Aineer.iconset")
try? FileManager.default.removeItem(at: iconsetDir)
try  FileManager.default.createDirectory(at: iconsetDir, withIntermediateDirectories: true)

let iconPairs: [(String, Int)] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]

for (name, sz) in iconPairs {
    if let png = drawAineerIcon(size: sz) {
        try png.write(to: iconsetDir.appendingPathComponent(name))
        print("  ✓ \(name) (\(sz)px)")
    }
}

// Create .icns
print("\nCreating .icns...")
let icnsPath = assetsDir.appendingPathComponent("aineer.icns")
let proc = Process()
proc.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
proc.arguments = ["-c", "icns", iconsetDir.path, "-o", icnsPath.path]
try proc.run()
proc.waitUntilExit()
if proc.terminationStatus == 0 {
    print("  ✓ aineer.icns")
} else {
    print("  ✗ iconutil failed (exit \(proc.terminationStatus))")
}

// Create Windows .ico (simplified: embed 256, 48, 32, 16 PNGs in ICO format)
print("\nCreating .ico...")
func createICO(sizes: [Int], outputPath: URL) -> Bool {
    var icoData = Data()
    let count = UInt16(sizes.count)

    // ICO header: reserved(2) + type(2) + count(2)
    icoData.append(contentsOf: [0, 0])          // reserved
    icoData.append(contentsOf: [1, 0])          // type = 1 (ICO)
    icoData.append(contentsOf: withUnsafeBytes(of: count.littleEndian) { Array($0) })

    var pngDataArray: [Data] = []
    for sz in sizes {
        if let png = drawAineerIcon(size: sz) {
            pngDataArray.append(png)
        } else {
            return false
        }
    }

    // Directory entries offset starts after header (6) + entries (16 * count)
    var offset = 6 + 16 * sizes.count

    for (i, sz) in sizes.enumerated() {
        let png = pngDataArray[i]
        let w = UInt8(sz >= 256 ? 0 : sz)
        let h = w
        icoData.append(w)                       // width
        icoData.append(h)                       // height
        icoData.append(0)                       // color palette
        icoData.append(0)                       // reserved
        icoData.append(contentsOf: [1, 0])      // color planes
        icoData.append(contentsOf: [32, 0])     // bits per pixel
        let pngSize = UInt32(png.count)
        icoData.append(contentsOf: withUnsafeBytes(of: pngSize.littleEndian) { Array($0) })
        let off32 = UInt32(offset)
        icoData.append(contentsOf: withUnsafeBytes(of: off32.littleEndian) { Array($0) })
        offset += png.count
    }

    for png in pngDataArray {
        icoData.append(png)
    }

    do {
        try icoData.write(to: outputPath)
        return true
    } catch {
        return false
    }
}

let icoPath = assetsDir.appendingPathComponent("aineer.ico")
if createICO(sizes: [256, 48, 32, 16], outputPath: icoPath) {
    print("  ✓ aineer.ico")
} else {
    print("  ✗ ICO creation failed")
}

print("\n✅ All icons generated at: \(assetsDir.path)")
