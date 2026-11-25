
            def a = {
                def b = {
                    if (x) return null      // bad
                    return [] as String[]
                }
                def c = {
                    if (x) return null      // bad
                    return [] as String[]
                }
                return [] as String[] // ok
            }
        