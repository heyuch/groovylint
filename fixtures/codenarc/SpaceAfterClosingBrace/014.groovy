
            def matching = list.find { it.isReady() }.filter()  // no violation for dot operator
            assert list.every { it.isReady() }, "Error"         // no violation for comma
            def m = [a:123, b:{ println 7 },c:99]               // no violation for comma
            processItems(list.select { it.isReady() })          // no violation for closing parenthesis
            maps.find { m -> m[index] }[index]                  // no violation for opening square bracket
            processItems([{ named("a") }, { named("b")}])       // no violation for closing square bracket
            def names = records.findAll { it.age > 1 }*.name    // no violation for spread operator
            parameters?.collect { it?.type?.toString() }?.join(', ')    // no violation for null-safe operator
            def closure = { println 7 };                        // no violation for comma
            writeLockLockInterceptor.tap { it.delegate = owner.delegate }()     // no violation for opening parenthesis
            switch(x) {
                case { x < 0 }:                                 // no violation for colon
                break
            }
        