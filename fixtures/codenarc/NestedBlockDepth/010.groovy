
            while (notReady()) {                // 1
                sleep(1000L)
                for(int i=0; i < 5; i++) {      // 2
                    println "waiting..."
                }
            }
            