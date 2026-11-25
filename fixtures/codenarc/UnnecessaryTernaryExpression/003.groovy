
            def x
            x = !ready ? true : false
            x = !(y + z) ? Boolean.TRUE : Boolean.FALSE
            x = (y == 99) ? false : true
            x = (y < 99) ? false : Boolean.TRUE
            x = (y <= 99) ? false : true
            x = (y > 99) ? true : false
            x = (y >= 99) ? false : true
            x = (y != 99) ? true : false
            x = (y ==~ /../) ? true : false
            x = (y && z) ? true : false
            x = (y || z) ? true : false
            x = (y || calculate(99)) ? true : false
            x = (addTax(5) + 5 || calculate(99) && ready) ? true : false
        