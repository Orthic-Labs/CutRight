import AppKit
import AVFoundation
import Foundation

/// Static, dependency-free bridge for the gated player spike. Rust owns the
/// handle & command policy; this object owns AVFoundation/AppKit lifetimes.
private final class CutRightPlayerHost {
  let player = AVPlayer()
  let layer = AVPlayerLayer()
  weak var hostView: NSView?

  init() { layer.player = player; layer.videoGravity = .resizeAspect }

  func attach(_ view: NSView) {
    detach()
    hostView = view
    view.wantsLayer = true
    view.layer?.addSublayer(layer)
    layer.frame = view.bounds
  }

  func setFrame(x: Double, y: Double, width: Double, height: Double) {
    guard let view = hostView else { return }
    // DOM coordinates start top-left; AppKit layer coordinates start bottom-left.
    layer.frame = CGRect(x: x, y: view.bounds.height - y - height, width: width, height: height)
  }

  func detach() { layer.removeFromSuperlayer(); hostView = nil }
  func load(_ path: String) { player.replaceCurrentItem(with: AVPlayerItem(url: URL(fileURLWithPath: path))) }
  func seek(_ numerator: Int64, _ denominator: Int32) { guard denominator > 0 else { return }; player.seek(to: CMTime(value: numerator, timescale: denominator), toleranceBefore: .zero, toleranceAfter: .zero) }
}

private func onMain<T>(_ body: () -> T) -> T {
  if Thread.isMainThread { return body() }
  return DispatchQueue.main.sync(execute: body)
}
private var scopedBookmarks: [UInt64: URL] = [:]
private var nextScopedBookmarkToken: UInt64 = 1

private func authorizedMediaPath(_ path: String, token: UInt64) -> String? {
  onMain {
    guard token != 0, let scope = scopedBookmarks[token] else { return nil }
    let media = URL(fileURLWithPath: path).resolvingSymlinksInPath().standardizedFileURL
    let root = scope.resolvingSymlinksInPath().standardizedFileURL
    guard (try? media.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true else { return nil }
    guard media.path == root.path || media.path.hasPrefix(root.path + "/") else { return nil }
    return media.path
  }
}

@_cdecl("cutright_player_create")
public func cutright_player_create() -> UnsafeMutableRawPointer {
  onMain { Unmanaged.passRetained(CutRightPlayerHost()).toOpaque() }
}

@_cdecl("cutright_player_destroy")
public func cutright_player_destroy(_ handle: UnsafeMutableRawPointer?) {
  guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).release() }
}

@_cdecl("cutright_player_load")
public func cutright_player_load(_ handle: UnsafeMutableRawPointer?, _ path: UnsafePointer<CChar>?, _ scopeToken: UInt64) -> Bool {
  guard let handle, let path else { return false }
  guard let authorizedPath = authorizedMediaPath(String(cString: path), token: scopeToken) else { return false }
  onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().load(authorizedPath) }; return true
}

@_cdecl("cutright_player_seek")
public func cutright_player_seek(_ handle: UnsafeMutableRawPointer?, _ numerator: Int64, _ denominator: Int32) -> Bool {
  guard let handle else { return false }
  guard denominator > 0 else { return false }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().seek(numerator, denominator) }; return true
}

@_cdecl("cutright_player_attach") public func cutright_player_attach(_ handle: UnsafeMutableRawPointer?, _ view: UnsafeMutableRawPointer?, _ x: Double, _ y: Double, _ width: Double, _ height: Double) -> Bool { guard let handle, let view else { return false }; onMain { let host = Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue(); host.attach(Unmanaged<NSView>.fromOpaque(view).takeUnretainedValue()); host.setFrame(x: x, y: y, width: width, height: height) }; return true }
@_cdecl("cutright_player_resize") public func cutright_player_resize(_ handle: UnsafeMutableRawPointer?, _ x: Double, _ y: Double, _ width: Double, _ height: Double) { guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().setFrame(x: x, y: y, width: width, height: height) } }
@_cdecl("cutright_player_play") public func cutright_player_play(_ handle: UnsafeMutableRawPointer?) { guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().player.play() } }
@_cdecl("cutright_player_pause") public func cutright_player_pause(_ handle: UnsafeMutableRawPointer?) { guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().player.pause() } }
@_cdecl("cutright_player_set_rate") public func cutright_player_set_rate(_ handle: UnsafeMutableRawPointer?, _ value: Float) { guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().player.rate = value } }
@_cdecl("cutright_player_set_volume") public func cutright_player_set_volume(_ handle: UnsafeMutableRawPointer?, _ value: Float) { guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().player.volume = value } }
@_cdecl("cutright_player_current_time") public func cutright_player_current_time(_ handle: UnsafeMutableRawPointer?) -> Double { guard let handle else { return 0 }; return onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().player.currentTime().seconds } }
@_cdecl("cutright_player_duration") public func cutright_player_duration(_ handle: UnsafeMutableRawPointer?) -> Double { guard let handle else { return 0 }; return onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().player.currentItem?.duration.seconds ?? 0 } }
@_cdecl("cutright_player_detach") public func cutright_player_detach(_ handle: UnsafeMutableRawPointer?) { guard let handle else { return }; onMain { Unmanaged<CutRightPlayerHost>.fromOpaque(handle).takeUnretainedValue().detach() } }

@_cdecl("cutright_bookmark_create")
public func cutright_bookmark_create(_ path: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
  guard let path else { return nil }
  let url = URL(fileURLWithPath: String(cString: path))
  guard let data = try? url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil) else { return nil }
  return strdup(data.base64EncodedString())
}

@_cdecl("cutright_bookmark_resolve")
public func cutright_bookmark_resolve(_ encoded: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
  guard let encoded, let data = Data(base64Encoded: String(cString: encoded)) else { return nil }
  var stale = false
  guard let url = try? URL(resolvingBookmarkData: data, options: .withSecurityScope, relativeTo: nil, bookmarkDataIsStale: &stale), url.startAccessingSecurityScopedResource() else { return nil }
  let refreshed = stale
    ? (try? url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil).base64EncodedString())
    : nil
  return onMain {
    let token = nextScopedBookmarkToken; nextScopedBookmarkToken += 1; scopedBookmarks[token] = url
    let refreshedValue = refreshed ?? ""
    return strdup("\(token)\n\(url.path)\n\(stale ? 1 : 0)\n\(refreshedValue)")
  }
}

@_cdecl("cutright_bookmark_release") public func cutright_bookmark_release(_ token: UInt64) { onMain { scopedBookmarks.removeValue(forKey: token)?.stopAccessingSecurityScopedResource() } }

@_cdecl("cutright_string_free") public func cutright_string_free(_ value: UnsafeMutablePointer<CChar>?) { if let value { free(value) } }
