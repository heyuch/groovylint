class SystemRunFinalizersOnExitClass3 {
                def myMethod() {
                    otherObject.runFinalizersOnExit(true)
                    System.out.println "1234"
                }
            }