
            if (ready) {                // 1
                if (logging) {          // 2
                    println "waiting..."
                } else {                // 2
                    start()
                }
            }
            