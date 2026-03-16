#include "orderflow.h"

#include <stdio.h>
#include <string.h>

int main(void) {
  of_engine_config_t cfg;
  memset(&cfg, 0, sizeof(cfg));
  cfg.instance_id = "c-basic";
  cfg.enable_persistence = 0;
  cfg.audit_max_bytes = 10ULL * 1024ULL * 1024ULL;
  cfg.audit_max_files = 5;
  cfg.audit_redact_tokens_csv = "secret,password,token,api_key";
  cfg.data_retention_max_bytes = 10ULL * 1024ULL * 1024ULL;
  cfg.data_retention_max_age_secs = 7ULL * 24ULL * 60ULL * 60ULL;

  of_engine_t* engine = NULL;
  int32_t rc = of_engine_create(&cfg, &engine);
  if (rc != OF_OK) {
    fprintf(stderr, "of_engine_create failed: %d\n", rc);
    return 1;
  }

  rc = of_engine_start(engine);
  if (rc != OF_OK) {
    fprintf(stderr, "of_engine_start failed: %d\n", rc);
    of_engine_destroy(engine);
    return 1;
  }

  printf("Orderflow C ABI started\n");
  printf("api_version=%u\n", of_api_version());
  printf("build_info=%s\n", of_build_info());

  (void)of_engine_poll_once(engine, OF_DQ_NONE);
  (void)of_engine_stop(engine);
  of_engine_destroy(engine);
  return 0;
}
