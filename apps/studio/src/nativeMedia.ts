import { call } from "./lib/api";

type RationalTime = { numerator: number; denominator: number };

export type MacMediaCapabilities = {
  avFoundation: boolean; vision: boolean; caption: boolean; preview: boolean;
  audio: boolean; metal: boolean; osVersion: string; workerVersion: string; workerBlake3: string;
};
export type NativeAssetInfo = {
  duration?: RationalTime | null;
  videoTracks: unknown[];
  audioTracks: unknown[];
};
export type NativeFrameRequest = {
  sourcePath: string;
  sourceFrameIndex: number;
  timestamp: RationalTime;
  sequenceId?: string | null;
  orientation?: string | null;
};
export type NativeFrameAnalysis = {
  sourceFrameIndex: number;
  timestamp: RationalTime;
  orientationTransform: string;
  visionRevision: number;
  faces: unknown[];
  bodies: unknown[];
  ocrBoxes: unknown[];
  saliency?: unknown;
};
export type NativeRenderArtifact = {
  outputPath: string; width: number; height: number; colorSpace: string; renderer: string;
};
export type NativeAudioFeatures = {
  sampleRate: number; channelCount: number; sampleCount: number; rms: number; peak: number;
  zeroCrossingRate: number; spectralFlux: number; envelope: number[];
  classification?: string | null; classificationConfidence?: number | null; classifierRevision?: string | null;
};

export const nativeMediaCapabilities = () =>
  call<MacMediaCapabilities>("native_media_capabilities");
export const inspectNativeAsset = (requestId: string, scopeToken: number, source: string) =>
  call<NativeAssetInfo>("native_media_inspect_asset", { requestId, scopeToken, source });
export const analyzeNativeFrames = (requestId: string, scopeToken: number, frames: NativeFrameRequest[]) =>
  call<NativeFrameAnalysis[]>("native_media_analyze_frames", {
    requestId, scopeToken, request: { frames, allowedRoots: [] },
  });
export const renderNativeCaption = (
  requestId: string, scopeToken: number,
  request: { outputPath: string; width: number; height: number; text: string; vertical: boolean },
) => call<NativeRenderArtifact>("native_media_render_caption", {
  requestId, scopeToken, request: { ...request, allowedRoots: [] },
});
export const renderNativePreview = (
  requestId: string, inputScopeToken: number, outputScopeToken: number,
  request: { inputPath: string; outputPath: string; cropX?: number; cropY?: number; cropWidth?: number; cropHeight?: number; rotationDegrees?: number },
) => call<NativeRenderArtifact>("native_media_render_preview", {
  requestId, inputScopeToken, outputScopeToken, request: { ...request, allowedRoots: [] },
});
export const readNativeAudioFeatures = (
  requestId: string, scopeToken: number,
  request: { sourcePath: string; startSeconds?: number; durationSeconds?: number },
) => call<NativeAudioFeatures>("native_media_audio_features", {
  requestId, scopeToken, request: { ...request, allowedRoots: [] },
});
export const cancelNativeMediaRequest = (requestId: string) =>
  call<void>("native_media_cancel", { requestId });
