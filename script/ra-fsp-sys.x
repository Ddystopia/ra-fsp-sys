INCLUDE memory_regions_base.ld
INCLUDE device.x /* PACs default handlers for IV */

PROVIDE(__ebss = __bss_end__);
PROVIDE(_stack_start = __stack);
PROVIDE(DefaultHandler = Default_Handler);
PROVIDE(SysTick = Default_Handler);

FLASH_START  = 0x00000000;
FLASH_LENGTH = 0x00200000;

UPDATE_FLASH_START  =  0x00200000;
UPDATE_FLASH_LENGTH =  0x00000000;

STATICFS_START  =   0x00000000;
STATICFS_LENGTH =   0x00000000;

INCLUDE fsp_base.ld

