
            int i = ~2
            int j = -1
            int z = +2
            boolean b = !true
            boolean c = !false

            int i = ~-2     // KNOWN LIMITATION; parses as BitwiseNegation+Constant
            int j = ~+9     // KNOWN LIMITATION; parses as BitwiseNegation+Constant
        