
            // here is the proper way to wait:
            countDownLatch.await()

            // this is weird code to write, but does not cause a violation
            // sleep inside a for-each loop is OK for some reason
            for (def x : collections) {
                sleep(1000)
            }

            for (def x = 0; x < 10; x++) {
                sleep(1000, 1, 3)   // sleep does not take 3 parameters
            }

            while (x) {
                // you should use a lock here, but technically you are
                // not just busy waiting because you are doing other work
                doSomething()
                sleep(1000)
            }
        