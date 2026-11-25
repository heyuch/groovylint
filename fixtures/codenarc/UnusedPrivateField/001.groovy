
            class Injected {}

            class Bean {
                private final Injected unused

                Bean(Injected unused) {
                    this.unused = unused
                }
            }
        