
            def doStuff() {
                for (int i = 0; i < 8; i++) {
                    outputMap.putAll(CopanSystemService.canisterInfo(suuid[i], i))
                }
            }
        