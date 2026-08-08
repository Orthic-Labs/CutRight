// CutRight's headless HeardRight speech adapter.
//
// The C ABI below is the minimal offline-ASR subset of sherpa-onnx c-api.h at
// 142807252687d81b40d6315f23470a1512a00de3. The packaged dylib is the exact
// universal binary recorded by vendored HeardRight. All model paths are
// relative to HR_MODELS_DIR, which CutRight sets from its verified speech pack.

#include <ctype.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { int32_t sample_rate, feature_dim; } SherpaOnnxFeatureConfig;
typedef struct { const char *encoder, *decoder, *joiner; } SherpaOnnxOfflineTransducerModelConfig;
typedef struct { const char *model; } OneModel;
typedef struct {
  const char *encoder, *decoder, *language, *task;
  int32_t tail_paddings, enable_token_timestamps, enable_segment_timestamps;
} WhisperModel;
typedef struct { const char *encoder, *decoder, *src_lang, *tgt_lang; int32_t use_pnc; } CanaryModel;
typedef struct { const char *encoder, *decoder, *language; int32_t use_punct, use_itn; } CohereModel;
typedef struct { const char *preprocessor, *encoder, *uncached_decoder, *cached_decoder, *merged_decoder; } MoonshineModel;
typedef struct { const char *model, *language; int32_t use_itn; } SenseVoiceModel;
typedef struct {
  const char *encoder_adaptor, *llm, *embedding, *tokenizer, *system_prompt,
      *user_prompt;
  int32_t max_new_tokens;
  float temperature, top_p;
  int32_t seed;
  const char *language;
  int32_t itn;
  const char *hotwords;
} FunAsrNanoModel;
typedef struct {
  const char *conv_frontend, *encoder, *decoder, *tokenizer;
  int32_t max_total_len, max_new_tokens;
  float temperature, top_p;
  int32_t seed;
  const char *hotwords;
} Qwen3Model;
typedef struct {
  SherpaOnnxOfflineTransducerModelConfig transducer;
  OneModel paraformer, nemo_ctc;
  WhisperModel whisper;
  OneModel tdnn;
  const char *tokens;
  int32_t num_threads, debug;
  const char *provider, *model_type, *modeling_unit, *bpe_vocab, *telespeech_ctc;
  SenseVoiceModel sense_voice;
  MoonshineModel moonshine;
  struct { const char *encoder, *decoder; } fire_red_asr;
  OneModel dolphin, zipformer_ctc;
  CanaryModel canary;
  OneModel wenet_ctc, omnilingual, medasr;
  FunAsrNanoModel funasr_nano;
  OneModel fire_red_asr_ctc;
  Qwen3Model qwen3_asr;
  CohereModel cohere_transcribe;
} SherpaOnnxOfflineModelConfig;
typedef struct { const char *model; float scale; } SherpaOnnxOfflineLMConfig;
typedef struct { const char *dict_dir, *lexicon, *rule_fsts; } SherpaOnnxHomophoneReplacerConfig;
typedef struct {
  SherpaOnnxFeatureConfig feat_config;
  SherpaOnnxOfflineModelConfig model_config;
  SherpaOnnxOfflineLMConfig lm_config;
  const char *decoding_method;
  int32_t max_active_paths;
  const char *hotwords_file;
  float hotwords_score;
  const char *rule_fsts, *rule_fars;
  float blank_penalty;
  SherpaOnnxHomophoneReplacerConfig hr;
} SherpaOnnxOfflineRecognizerConfig;
typedef struct SherpaOnnxOfflineRecognizer SherpaOnnxOfflineRecognizer;
typedef struct SherpaOnnxOfflineStream SherpaOnnxOfflineStream;
typedef struct {
  const char *text;
  float *timestamps;
  int32_t count;
  const char *tokens;
  const char *const *tokens_arr;
  const char *json, *lang, *emotion, *event;
  float *durations, *ys_log_probs;
  const float *segment_timestamps, *segment_durations;
  const char *segment_texts;
  const char *const *segment_texts_arr;
  int32_t segment_count;
} SherpaOnnxOfflineRecognizerResult;
typedef struct { const float *samples; int32_t sample_rate, num_samples; } SherpaOnnxWave;

extern const SherpaOnnxOfflineRecognizer *SherpaOnnxCreateOfflineRecognizer(
    const SherpaOnnxOfflineRecognizerConfig *);
extern void SherpaOnnxDestroyOfflineRecognizer(const SherpaOnnxOfflineRecognizer *);
extern const SherpaOnnxOfflineStream *SherpaOnnxCreateOfflineStream(
    const SherpaOnnxOfflineRecognizer *);
extern void SherpaOnnxDestroyOfflineStream(const SherpaOnnxOfflineStream *);
extern void SherpaOnnxAcceptWaveformOffline(const SherpaOnnxOfflineStream *,
                                            int32_t, const float *, int32_t);
extern void SherpaOnnxDecodeOfflineStream(const SherpaOnnxOfflineRecognizer *,
                                          const SherpaOnnxOfflineStream *);
extern const SherpaOnnxOfflineRecognizerResult *SherpaOnnxGetOfflineStreamResult(
    const SherpaOnnxOfflineStream *);
extern void SherpaOnnxDestroyOfflineRecognizerResult(
    const SherpaOnnxOfflineRecognizerResult *);
extern const SherpaOnnxWave *SherpaOnnxReadWave(const char *);
extern void SherpaOnnxFreeWave(const SherpaOnnxWave *);

static SherpaOnnxOfflineRecognizer const *recognizer;
static char encoder[4096], decoder[4096], joiner[4096], tokens[4096];

static int json_string(const char *json, const char *key, char *out, size_t cap) {
  char needle[128];
  snprintf(needle, sizeof needle, "\"%s\"", key);
  const char *p = strstr(json, needle);
  if (!p || !(p = strchr(p + strlen(needle), ':'))) return 0;
  while (isspace((unsigned char)*++p)) {}
  if (*p++ != '"') return 0;
  size_t n = 0;
  while (*p && *p != '"' && n + 1 < cap) {
    if (*p == '\\') {
      p++;
      if (!*p) break;
      if (*p == 'n') out[n++] = '\n';
      else if (*p == 'r') out[n++] = '\r';
      else if (*p == 't') out[n++] = '\t';
      else if (*p == 'u') { out[n++] = '?'; p += strspn(p + 1, "0123456789abcdefABCDEF"); }
      else out[n++] = *p;
      p++;
    } else out[n++] = *p++;
  }
  out[n] = 0;
  return *p == '"';
}

static void json_print(const char *s) {
  putchar('"');
  for (; s && *s; s++) {
    unsigned char c = (unsigned char)*s;
    if (c == '"' || c == '\\') { putchar('\\'); putchar(c); }
    else if (c == '\n') fputs("\\n", stdout);
    else if (c == '\r') fputs("\\r", stdout);
    else if (c == '\t') fputs("\\t", stdout);
    else if (c >= 0x20) putchar(c);
  }
  putchar('"');
}

static void error_frame(const char *request_id, const char *trace_id,
                        const char *code, const char *message) {
  fputs("{\"protocol_major\":1,\"protocol_minor\":0,\"schema_name\":\"engine_error\",\"schema_version\":1,\"engine_version\":\"cutright-heardright-sherpa/1\",\"request_id\":", stdout);
  json_print(request_id);
  fputs(",\"trace_id\":", stdout); json_print(trace_id);
  fputs(",\"error\":{\"code\":", stdout); json_print(code);
  fputs(",\"message\":", stdout); json_print(message);
  fputs("}}\n", stdout);
}

static int init_recognizer(char *error, size_t cap) {
  if (recognizer) return 1;
  const char *root = getenv("HR_MODELS_DIR");
  if (!root || !*root) {
    snprintf(error, cap, "runtime_pack_unavailable: HR_MODELS_DIR is absent");
    return 0;
  }
  snprintf(encoder, sizeof encoder, "%s/encoder.int8.onnx", root);
  snprintf(decoder, sizeof decoder, "%s/decoder.int8.onnx", root);
  snprintf(joiner, sizeof joiner, "%s/joiner.int8.onnx", root);
  snprintf(tokens, sizeof tokens, "%s/tokens.txt", root);
  FILE *files[4] = {fopen(encoder, "rb"), fopen(decoder, "rb"),
                    fopen(joiner, "rb"), fopen(tokens, "rb")};
  const long sizes[4] = {652184281L, 11845275L, 6355277L, 93939L};
  for (int i = 0; i < 4; i++) {
    if (!files[i] || fseek(files[i], 0, SEEK_END) || ftell(files[i]) != sizes[i]) {
      for (int j = 0; j < 4; j++) if (files[j]) fclose(files[j]);
      snprintf(error, cap, "runtime_pack_unavailable: speech payload is absent or invalid");
      return 0;
    }
    fclose(files[i]);
  }
  SherpaOnnxOfflineRecognizerConfig cfg;
  memset(&cfg, 0, sizeof cfg);
  cfg.feat_config.sample_rate = 16000;
  cfg.feat_config.feature_dim = 128;
  cfg.model_config.transducer.encoder = encoder;
  cfg.model_config.transducer.decoder = decoder;
  cfg.model_config.transducer.joiner = joiner;
  cfg.model_config.tokens = tokens;
  cfg.model_config.num_threads = 2;
  cfg.model_config.provider = "cpu";
  cfg.model_config.model_type = "nemo_transducer";
  cfg.decoding_method = "greedy_search";
  recognizer = SherpaOnnxCreateOfflineRecognizer(&cfg);
  if (!recognizer) {
    snprintf(error, cap, "runtime_pack_unavailable: speech payload failed validation");
    return 0;
  }
  return 1;
}

static const char *token_text(const char *token) {
  if (!token) return "";
  if ((unsigned char)token[0] == 0xe2 && (unsigned char)token[1] == 0x96 &&
      (unsigned char)token[2] == 0x81) return token + 3;
  while (*token == ' ') token++;
  return token;
}

static int token_starts_word(const char *token) {
  return token && (*token == ' ' ||
                    ((unsigned char)token[0] == 0xe2 &&
                     (unsigned char)token[1] == 0x96 &&
                     (unsigned char)token[2] == 0x81));
}

static void print_word(const char *word, float start, float end, int *written) {
  if (!*word) return;
  if ((*written)++) putchar(',');
  fputs("{\"text\":", stdout); json_print(word);
  printf(",\"start_ms\":%lld,\"end_ms\":%lld}",
         (long long)(start * 1000.0f), (long long)(end * 1000.0f));
}

static void transcribe(const char *request_id, const char *trace_id,
                       const char *path) {
  char error[256];
  if (!init_recognizer(error, sizeof error)) {
    error_frame(request_id, trace_id, "runtime_pack_unavailable", error);
    return;
  }
  const SherpaOnnxWave *wave = SherpaOnnxReadWave(path);
  if (!wave) {
    error_frame(request_id, trace_id, "unsupported_media",
                "speech input must be readable mono PCM WAVE");
    return;
  }
  const SherpaOnnxOfflineStream *stream = SherpaOnnxCreateOfflineStream(recognizer);
  if (!stream) {
    SherpaOnnxFreeWave(wave);
    error_frame(request_id, trace_id, "engine_failure", "could not create ASR stream");
    return;
  }
  SherpaOnnxAcceptWaveformOffline(stream, wave->sample_rate, wave->samples,
                                  wave->num_samples);
  SherpaOnnxDecodeOfflineStream(recognizer, stream);
  const SherpaOnnxOfflineRecognizerResult *result =
      SherpaOnnxGetOfflineStreamResult(stream);
  if (!result) {
    SherpaOnnxDestroyOfflineStream(stream);
    SherpaOnnxFreeWave(wave);
    error_frame(request_id, trace_id, "engine_failure", "ASR returned no result");
    return;
  }

  fputs("{\"protocol_major\":1,\"protocol_minor\":0,\"schema_name\":\"file_transcription_result\",\"schema_version\":1,\"engine_version\":\"cutright-heardright-sherpa/1\",\"request_id\":", stdout);
  json_print(request_id); fputs(",\"trace_id\":", stdout); json_print(trace_id);
  fputs(",\"payload\":{\"kind\":\"file_transcription_result\",\"text\":", stdout);
  json_print(result->text ? result->text : "");
  fputs(",\"srt\":\"\",\"vtt\":\"\",\"words\":[", stdout);
  int written = 0;
  char word[1024] = "";
  float word_start = 0.0f, word_end = 0.0f;
  for (int32_t i = 0; i < result->count; i++) {
    const char *raw = result->tokens_arr ? result->tokens_arr[i] : "";
    const char *piece = token_text(raw);
    if (!*piece) continue;
    float start = result->timestamps ? result->timestamps[i] : i * 0.04f;
    float end = result->durations ? start + result->durations[i]
                                  : (i + 1 < result->count && result->timestamps
                                         ? result->timestamps[i + 1]
                                         : start + 0.04f);
    if (end <= start) end = start + 0.04f;
    if (token_starts_word(raw) && *word) {
      print_word(word, word_start, word_end, &written);
      word[0] = 0;
    }
    if (!*word) word_start = start;
    strncat(word, piece, sizeof word - strlen(word) - 1);
    word_end = end;
  }
  print_word(word, word_start, word_end, &written);
  fputs("]}}\n", stdout);
  SherpaOnnxDestroyOfflineRecognizerResult(result);
  SherpaOnnxDestroyOfflineStream(stream);
  SherpaOnnxFreeWave(wave);
}

int main(void) {
  setvbuf(stdout, NULL, _IONBF, 0);
  char *line = NULL;
  size_t size = 0;
  while (getline(&line, &size, stdin) >= 0) {
    char schema[128] = "", request_id[512] = "", trace_id[512] = "", path[4096] = "";
    if (!json_string(line, "schema_name", schema, sizeof schema) ||
        !json_string(line, "request_id", request_id, sizeof request_id)) {
      error_frame("", "", "invalid_request", "request frame is missing schema_name or request_id");
      continue;
    }
    json_string(line, "trace_id", trace_id, sizeof trace_id);
    if (!strcmp(schema, "session_handshake_request")) {
      fputs("{\"protocol_major\":1,\"protocol_minor\":0,\"schema_name\":\"session_handshake_result\",\"schema_version\":1,\"engine_version\":\"cutright-heardright-sherpa/1\",\"request_id\":", stdout);
      json_print(request_id); fputs(",\"trace_id\":", stdout); json_print(trace_id);
      fputs(",\"payload\":{\"kind\":\"session_handshake_result\",\"capabilities\":[\"file_transcription_v1\"]}}\n", stdout);
    } else if (!strcmp(schema, "file_transcription_request") &&
               json_string(line, "path", path, sizeof path)) {
      transcribe(request_id, trace_id, path);
    } else {
      error_frame(request_id, trace_id, "unsupported_request", "engine supports file_transcription_v1 only");
    }
  }
  free(line);
  if (recognizer) SherpaOnnxDestroyOfflineRecognizer(recognizer);
  return 0;
}
