ifeq ($(TARGET_BUILD_APPS),)

.PHONY: systemlicense
systemlicense: $(call corresponding-license-metadata, $(SYSTEM_NOTICE_DEPS)) reportmissinglicenses

ifneq (,$(SYSTEM_NOTICE_DEPS))
SYSTEM_NOTICE_DEPS += $(UNMOUNTED_NOTICE_DEPS) $(UNMOUNTED_NOTICE_VENDOR_DEPS)
$(eval $(call text-notice-rule,$(target_notice_file_txt),"System image",$(system_notice_file_message),$(SYSTEM_NOTICE_DEPS),$(SYSTEM_NOTICE_DEPS)))
endif

endif # not TARGET_BUILD_APPS
