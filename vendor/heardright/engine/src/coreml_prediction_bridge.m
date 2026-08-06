#import <CoreML/CoreML.h>

// Keep Core ML's Objective-C exception entirely inside an Objective-C frame.
// Every non-null returned object is retained (+1) for Rust to adopt.
void *heardright_coreml_prediction(void *model_ptr,
                                   void *provider_ptr,
                                   void **error_out,
                                   void **exception_description_out) {
    if (error_out) {
        *error_out = NULL;
    }
    if (exception_description_out) {
        *exception_description_out = NULL;
    }

    MLModel *model = (__bridge MLModel *)model_ptr;
    id<MLFeatureProvider> provider = (__bridge id<MLFeatureProvider>)provider_ptr;
    @try {
        NSError *error = nil;
        id<MLFeatureProvider> result = [model predictionFromFeatures:provider error:&error];
        if (result) {
            return (__bridge_retained void *)result;
        }
        if (error_out && error) {
            *error_out = (__bridge_retained void *)error;
        }
        return NULL;
    } @catch (NSException *exception) {
        if (exception_description_out) {
            NSString *reason = exception.reason ?: @"<no reason>";
            NSString *description = [NSString stringWithFormat:@"%@: %@", exception.name, reason];
            *exception_description_out = (__bridge_retained void *)description;
        }
        return NULL;
    }
}

// Release-mode smoke for the Rust harness. This must return instead of letting
// an Objective-C exception cross the C ABI boundary.
void *heardright_coreml_exception_smoke(void) {
    @try {
        @throw [NSException exceptionWithName:@"HeardRightCoreMLSmoke"
                                       reason:@"exception containment smoke"
                                     userInfo:nil];
    } @catch (NSException *exception) {
        NSString *description =
            [NSString stringWithFormat:@"%@: %@", exception.name, exception.reason];
        return (__bridge_retained void *)description;
    }
}
