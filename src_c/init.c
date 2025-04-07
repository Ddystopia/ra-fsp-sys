void SystemInit(void);

void __pre_init(void) {

  // It will setup FPU too, but it doesn't matter

  SystemInit();
}
