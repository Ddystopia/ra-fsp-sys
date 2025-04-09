INCLUDE memory_regions_base.ld

FLASH_START  = 0x00000000;
FLASH_LENGTH = 0x001FFF00;

/* define UPDATE_FLASH_xxx to non-existent location, to allow
   standalone code to build, but fail update in runtime */
UPDATE_FLASH_START  =  0x00200000;
UPDATE_FLASH_LENGTH =  0x00000000;
/* Uncomment to allow firmware updates in standalone images
UPDATE_FLASH_START =   0x00150000;
UPDATE_FLASH_LENGTH =  0x000B0000;
*/

STATICFS_START  =   0x001FFF00;
STATICFS_LENGTH =   0x00000100;
