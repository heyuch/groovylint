
            class FakeIdGeneratorHelper {
                private static long id = 0
                private long negCounter = 0
                private long c1 = 0
                private long c2 = 0
                long generate(SessionImplementor session) {
                    println id++
                    println negCounter--
                    println (--c1)
                    println (--c2)
                }
            }
        