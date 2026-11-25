
            doWithTransactionAndReallyLongName(
                'testing within a transaction',
                new BigDecimal("123456789012345")) {
                    process('12345') }
        