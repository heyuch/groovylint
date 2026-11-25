
            class SystemRunFinalizersOnExitClass2 {
                static def method1() {
                    System.runFinalizersOnExit(true)
                }
                def method2() {
                    System.runFinalizersOnExit(true)
                }
            }
        