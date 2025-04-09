INCLUDE memory_regions_base.ld

/*
 * Following definitions must exactly match bootloader settings for this board
 */

FLASH_START         = 0x00010000 + 0x400;
UPDATE_FLASH_START  = 0x00150000;
FLASH_LENGTH        = UPDATE_FLASH_START - FLASH_START;
UPDATE_FLASH_LENGTH = 0x000B0000;

STATICFS_START      = 0x0000B700;
STATICFS_LENGTH     = 0x0004C900;
