
            class MyClass {
                def order1 = existingFunds.collect {it.fundSortOrder} ?: []
                def order2 = existingFunds.collect {it.fundSortOrder} ? [1] : [2]
                def order3 = { 23 } << { 37}
            }
        