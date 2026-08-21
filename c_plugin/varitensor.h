/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * Licence, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#ifndef VARITENSOR_H
#define VARITENSOR_H

#include <algorithm>
#include <cmath>
#include <cstring>
#include <functional>
#include <iomanip>
#include <map>
#include <memory>
#include <numeric>
#include <ranges>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <variant>
#include <vector>

#ifndef VARITENSOR_VALIDATION_ON
#define VARITENSOR_VALIDATION_ON 1
#endif

namespace varitensor {

class Tensor;
class View;
class Index;

/**
 * @brief Represents a logic error in the requested tensor operations
 *
 * This normally means a caller requested an operation that doesn't make sense mathematically, like trying to transpose
 * two indices of different sizes.
 */
struct TensorLogicError final: std::logic_error {
    explicit TensorLogicError(const std::string& message):
        std::logic_error("<Vari-Tensor> Tensor logic error: " + message)
    {}
};

namespace impl {

template <bool is_const>
class ViewIterator;

class LinkedOp;
class LinkedOpIterator;
class ProductOp;
class ProductOpIterator;
struct Preparatory;

constexpr int MAX_INTERVAL = -1;

// NB: we occasionally use the fact that ADD == -SUB
enum Operation {SUB=-1, ADD=1, MUL=2, DIV=3};

enum ExprState {
    SCALAR,
    ALIGNED_INDICES,
    FREE_MULTIPLICATION,
    GENERAL
};

enum PreparatoryType {
    LINKED,
    PRODUCT
};

inline void deny(const bool condition, const std::string& message) {
    if constexpr (VARITENSOR_VALIDATION_ON) {
        if (condition) [[unlikely]] {
            throw TensorLogicError(message);
        }
    }
}

inline auto VBegin = [](auto& expression) {return expression.vbegin();};
inline auto GetDimensions = [](auto& expression) {return expression.dimensions();};
inline auto GetSize = [](auto& expression) {return expression.size();};
inline auto GetData = [](auto& iter) {return iter.data();};

using DoublePtr = std::unique_ptr<double, void(*)(double*)>;

} // namespace impl

/**
 * @brief Helper alias to provide the correct signature for user-defined metric functions
 */
using MetricFunction = std::function<double(int, int)>;

/**
 * @brief Convenience aliases for setting index size
 */
enum IndexSizes {LATIN = 3, GREEK = 4};

/**
 * @brief Aliases used to express index variance
 */
enum Variance {COVARIANT = 0, LOWER = 0, CONTRAVARIANT = 1, UPPER = 1};

/**
 * @brief Expresses an interval on an index
 *
 * Both first and last are included in the range of the interval.
 */
struct Interval {
    const Index& origin;
    const int first;
    const int last;
};

/**
 * @brief Represents an index of a tensor
 */
class Index {
public:
/**
 * @brief Initialise an index
 *
 * @param size Index size; must be greater than 1.
 */
///@{
    explicit Index(const int size):
        m_size{size},
        m_id{s_next_id++}
    {
        impl::deny(m_size < 2, "Cannot initialize unnamed index with size < 2");
    }

    explicit Index(std::string  name, const int size):
        m_size{size},
        m_id{s_next_id++},
        m_name{std::move(name)}
    {
        impl::deny(m_size < 2, "Cannot initialize index with size < 2");
    }
///@}

/**
 * @brief Create a new index from an interval
 */
    Index(const Interval& interval): // NOLINT - we want implicit conversion
        m_size{interval.last - interval.first + 1},
        m_id{interval.origin.m_id},
        m_name{interval.origin.m_name},
        m_interval_start{interval.first}
    {}

/**
 * @brief Compare two indices
 *
 * Note that for indices the name is included in the comparison.
 */
    bool operator==(const Index& other) const {
        if (m_name.empty()) return m_id == other.m_id && m_size == other.m_size;
        return m_name == other.m_name && m_size == other.m_size;
    }

    [[nodiscard]] const int& size() const { return m_size; }

    [[nodiscard]] const int& id() const { return m_id; }

    [[nodiscard]] std::string name() const {
        std::ostringstream name;

        if (m_name.empty()) name << "idx" << m_id;
        else name << m_name;

        if (m_interval_start != -1) {
            name << std::string("(") << m_interval_start << '-' << (m_interval_start + m_size - 1) << ')';
        }

        return name.str();
    }

    void set_name(const std::string& name) { m_name = name; }

/**
 * @brief Use when slicing a tensor to select an interval of an index
 *
 * e.g. T[ index(1) ] will get a slice ignoring the 0th elements of T.
 *
 * @param first The first value to include; must be greater than 0.
 * @param last The last index to include; takes the maximum index value by default.
 */
    ///@{
    [[nodiscard]] Interval operator()(const int first, const int last=impl::MAX_INTERVAL) const {
        return interval(first, last);
    }

    [[nodiscard]] Interval interval(const int first, int last=impl::MAX_INTERVAL) const {
        if (last == impl::MAX_INTERVAL) last = m_size - 1;

        impl::deny(first < 0, "Interval cannot have negative start");
        impl::deny(first >= last, "Interval must end after it starts");
        impl::deny(last >= m_size, "Interval cannot overflow index size");
        return {*this, first, last};
    }
///@}

private:
    int m_size;
    int m_id;
    std::string m_name{};
    int m_interval_start{-1};

    inline static int s_next_id{0};
};

/**
 * @brief Represents an index with a variance
 *
 * Used when constructing tensors.
 */
struct VarianceQualifiedIndex {
    Index index;
    Variance variance = COVARIANT;

    bool operator==(const VarianceQualifiedIndex& other) const {
        return index == other.index && variance == other.variance;
    }
};

namespace impl {
struct Dimension {
    Index index;
    Variance variance{};
    size_t width{}; // the memory width of a single increment along this dimension

    [[nodiscard]] int size() const {return index.size();}
};

using Dimensions = std::vector<Dimension>;

template<typename T>
concept Expression_c = std::same_as<T, View> || std::same_as<T, LinkedOp> || std::same_as<T, ProductOp>;
using Expression = std::variant<View, LinkedOp, ProductOp>;
using Expressions = std::vector<Expression>;

template<typename T>
concept ExpressionIterator_c =
    std::same_as<T, ViewIterator<true>>  ||
    std::same_as<T, ViewIterator<false>> ||
    std::same_as<T, LinkedOpIterator>    ||
    std::same_as<T, ProductOpIterator>;
using ExpressionIterator = std::variant<ViewIterator<true>, ViewIterator<false>, LinkedOpIterator, ProductOpIterator>;
using ExpressionIterators = std::vector<ExpressionIterator>;

template <typename T>
concept ExpressionOperand_c =
    std::is_same_v<T, Tensor>   ||
    std::is_same_v<T, View>     ||
    std::is_same_v<T, LinkedOp> ||
    std::is_same_v<T, ProductOp> ||
    std::is_same_v<T, double>;

struct Reset {
    explicit Reset(const int index_id_): index_id{index_id_} {}

    template <ExpressionIterator_c E>
    void operator()(const E& iter) const {iter.reset(index_id);}

    int index_id;
};

struct Increment {
    explicit Increment(const int index_id_): index_id{index_id_} {}

    template <ExpressionIterator_c E>
    void operator()(const E& iter) const {iter.increment(index_id);}

    int index_id;
};

template<typename... Types>
concept AllInt_c = (... && std::is_same_v<Types, int>);

template<typename T>
concept Indexable_c = std::is_integral_v<T> || std::is_same_v<T, Index> || std::is_same_v<T, Interval>;

template<typename T>
concept Indices_c =
    std::is_same_v<std::decay_t<T>, std::vector<Index>> ||
    std::is_same_v<std::decay_t<T>, std::initializer_list<Index>> ||
    std::is_same_v<std::decay_t<T>, std::vector<VarianceQualifiedIndex>> ||
    std::is_same_v<std::decay_t<T>, std::initializer_list<VarianceQualifiedIndex>>;

template<typename T>
concept VarianceQualifiedIndices_c =
    std::is_same_v<std::decay_t<T>, std::vector<VarianceQualifiedIndex>> ||
    std::is_same_v<std::decay_t<T>, std::initializer_list<VarianceQualifiedIndex>>;

const std::string TENSOR_DEFAULT_NAME = "Vari-Tensor";

enum TensorClass {
    TENSOR,
    METRIC_TENSOR,
    KRONECKER_DELTA,
    LEVI_CIVITA_SYMBOL,
    CHRISTOFFEL_SYMBOL
};

/**
 * @brief Type aliases to help with indexing tensors
 */
///@{
using Indexable = std::variant<int, Index>;
using Indexables = std::vector<Indexable>;
///@}

void deallocate(double* data);
DoublePtr allocate(size_t size);

struct Preparatory {
    ExprState state{SCALAR};
    Dimensions dimensions;
    Dimensions repeated; // minor pollution from ProductOpIterator, but unlikely to affect performance
    ExpressionIterators sub_iterators;
    size_t size{1};

    Preparatory() = default;
    Preparatory(const Expressions& sub_expressions, PreparatoryType type);

    struct Couple {
        int dims[2]{-1, -1};
        int metric_index{-1};
        int other_index{-1};

        void add(const int dimension, const bool is_metric) {
            deny(dims[0] != -1 && dims[1] != -1,
                   "Indices in multiplication expression cannot appear more than twice");
            const auto target = dims[0] == -1 ? 0 : 1;
            dims[target] = dimension;
            if (!is_metric) other_index = dimension;
        }

        void clear() {
            dims[0] = -1;
            dims[1] = -1;
            metric_index = -1;
        }

        [[nodiscard]] bool has() const {
            return dims[0] != -1;
        }

        [[nodiscard]] bool is_repeated() const {
            return dims[1] != -1;
        }

        [[nodiscard]] bool is_metric() const {
            return metric_index != -1;
        }
    };

private:
    void prepare_linked_operation(const Expressions& sub_expressions);
    void prepare_product_operation(const Expressions& sub_expressions);
};

class ExpressionIteratorBase {
public:
    using iterator_category = std::forward_iterator_tag;
    using difference_type   = std::ptrdiff_t;
    using value_type        = double;

    ExpressionIteratorBase() = default;

    explicit ExpressionIteratorBase(Preparatory& preparatory):
        m_size{preparatory.size},
        m_dimensions{std::move(preparatory.dimensions)},
        m_positions(m_dimensions.size(), 0)
    {}

    [[nodiscard]] std::vector<int>& positions() {return m_positions;}

    [[nodiscard]] int positions(const int index) const {return m_positions[index];}

    [[nodiscard]] int positions(const Index& index) const {
        for (size_t i = 0; i < m_dimensions.size(); ++i) {
            if (m_dimensions[i].index == index) return m_positions[i];
        }
        throw TensorLogicError(
            "Unable to find index!"
        );  // can't use deny() as there is nothing to return
    }

    [[nodiscard]] const Dimensions& dimensions() const {return m_dimensions;}
    [[nodiscard]] size_t size() const {return m_size;}
    [[nodiscard]] bool is_scalar() const {return m_size == 1;}

protected:
    friend class varitensor::Tensor;

    size_t m_size{1};
    Dimensions m_dimensions;
    std::vector<int> m_positions;
};

template<bool is_const=false>
class ViewIterator: public ExpressionIteratorBase {
public:
    ViewIterator() = default;
    ViewIterator(const Tensor* target, double* data_ptr, const Dimensions& dimensions):
        m_target{target},
        m_data_ptr{data_ptr}
    {
        std::unordered_map<int, int> counts;
        for (auto& dimension: dimensions) counts[dimension.index.id()] += 1;

        for (auto& dimension: dimensions) {
            deny(counts[dimension.index.id()] > 2,
                      "Indices in contraction expressions cannot appear more than twice");

            if (counts[dimension.index.id()] == 2) { // repeated index
                m_repeated.push_back(dimension);
                m_repeated_positions.push_back(0);
                counts[dimension.index.id()] = -1;
            }
            else if (counts[dimension.index.id()] == 1) { // non-repeated index
                m_dimensions.push_back(dimension);
                m_size *= dimension.size();
                m_positions.push_back(0);
            }

            m_widths[dimension.index.id()] += {
                dimension.width,
                dimension.width * (dimension.size() -1)
            };
        }

        if (!m_dimensions.empty()) {
            m_cached_id = m_dimensions.front().index.id();
            m_cached_info = m_widths[m_cached_id];
        }
    }

    ViewIterator(const ViewIterator& other) = default;
    ViewIterator& operator=(const ViewIterator& other) = default;
    ViewIterator(ViewIterator&& other) noexcept = default;
    ViewIterator& operator=(ViewIterator&& other) = default;

    ViewIterator& operator++() {
        if (!increment_positions(m_positions, m_dimensions, *this)) {
            m_data_ptr = nullptr;
        }
        return *this;
    }

    ViewIterator operator++(int) {
        ViewIterator copy = *this;
        ++*this;
        return copy;
    }

    double& operator*() const requires(!is_const) {
        deny(is_contracted(),
             "Cannot dereference non-const contracted iterator - use std::as_const or cbegin/cend for const iteration");
        return *m_data_ptr;
    }

    double operator*() const requires(is_const) {
        return deref();
    }


    template<typename T>
    requires std::is_same_v<T, ViewIterator<>> || std::is_same_v<T, ViewIterator<true>>
    bool operator==(const T& other) const {
        return m_data_ptr == other.m_data_ptr;
    }

    [[nodiscard]] bool is_contracted() const { // used to enforce the fact that contractions can only be r-values
        return !m_repeated.empty();
    }

    bool is_metric() const;

    [[nodiscard]] bool finished() const {
        return m_data_ptr == nullptr;
    }

    void increment(const int index_id) { // NOLINT - "increment()" const is only pseudo-const
        std::as_const(*this).increment(index_id); // std::as_const needed to avoid recursion
    }

    void reset(const int index_id) { // NOLINT - "reset() const" is only pseudo-const
        std::as_const(*this).reset(index_id); // std::as_const needed to avoid recursion
    }

    [[nodiscard]] double deref() const {
        /* Despite modifying m_repeated_positions, this function is const as it always returns the positions to 0 */
        double sum = 0;

        do sum += *m_data_ptr;
        while (increment_positions(m_repeated_positions, m_repeated, *this));

        return sum;
    }

    bool is_contiguous() const {
        if (is_contracted()) return false; // contracted views counted as non-contiguous

        size_t expected_width = 1;
        for (const auto& dimension: m_dimensions) {
            if (dimension.width != expected_width) return false;
            expected_width *= dimension.size();
        }
        return true;
    }

    [[nodiscard]] double* data() const {
        return m_data_ptr;
    }

private:
    struct WidthInfo {
        size_t width{0};
        size_t total{0};

        WidthInfo& operator+=(WidthInfo&& other) {
            width += other.width;
            total += other.total;
            return *this;
        }
    };

    void increment(const int index_id) const { // technically non-const but has to be callable from deref()
        if (index_id == m_cached_id) [[likely]] m_data_ptr += m_cached_info.width;
        else if (m_widths.contains(index_id)) m_data_ptr += m_widths.at(index_id).width;
    }

    void reset(const int index_id) const { // technically non-const but has to be callable from deref()
        if (index_id == m_cached_id) [[likely]] m_data_ptr -= m_cached_info.total;
        else if (m_widths.contains(index_id)) m_data_ptr -= m_widths.at(index_id).total;
    }

    friend class ViewIterator<!is_const>;
    friend struct Reset;
    friend struct Increment;

    template <ExpressionIterator_c E>
    friend bool increment_positions(std::vector<int>& positions, const std::vector<Dimension>& dimensions, const E& iterator);

    const Tensor* m_target{nullptr};
    mutable double* m_data_ptr{nullptr}; // mutable for deref

    std::map<int, WidthInfo> m_widths;

    // profiling has indicated that map lookup can be a bottleneck: these caches are used to reduce this
    int m_cached_id{-1};
    WidthInfo m_cached_info;

    // for index contraction
    Dimensions m_repeated;
    mutable std::vector<int> m_repeated_positions; // mutable for deref
};

} // namespace impl

/**
 * @brief Represents a view on a tensor using the provided indices
 */
class View {
public:
/**
 * @brief Initialise a plain view on a tensor
 */
    explicit View(const Tensor& target); // a plain view on a tensor
    View(const Tensor& target, double* data_ptr, impl::Dimensions dimensions); // an offset view

/**
 * @brief Assign a slice of a tensor to take the values from another tensor
 */
    void operator=(const Tensor& other) &&; // NOLINT - allows T[i, 2] = U

    template<typename T = void>
    void operator=(const Tensor&) const & { // NOLINT - this definition only exists to document incorrect syntax
        static_assert(!std::is_same_v<T, T>,
            "View assignment is only valid on rvalue Views (e.g. tensor[i, j] = other). "
            "Did you mean to assign to the underlying Tensor instead?");
    }

    bool operator==(const View& other) const;

    using iterator = impl::ViewIterator<>;
    using const_iterator = impl::ViewIterator<true>;

/**
 * @brief Standard iterator and const iterator functions
 */
///@{
    [[nodiscard]] iterator begin();
    [[nodiscard]] iterator end();
    [[nodiscard]] const_iterator begin() const;
    [[nodiscard]] const_iterator end() const;
    [[nodiscard]] const_iterator cbegin() const;
    [[nodiscard]] const_iterator cend() const;
///@}

    [[nodiscard]] impl::ExpressionIterator vbegin() const;

    [[nodiscard]] double* data() const;
    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;

    void populate(Tensor& tensor, bool allocate = true) const;

private:
    const Tensor& m_target;
    double* m_data_ptr; // allows us to store an offset
    impl::Dimensions m_dimensions;
};

/**
 * @brief Represents a generalised mathematical tensor
 *
 * This class is capable of representing tensors of any rank, including scalars. It can also represent metric tensors
 * when created via varitensor::metric_tensor().
 *
 * The dimensions of a tensor are always aware of their variance, though the user can ignore this, in which case the
 * variance defaults to covariant.
 */
class Tensor {
public:
/**
 * @brief Construct a tensor with a collection of indices
 *
 * Each constructor version can be called with a name, index information, and/or initial fill value (0 by default),
 * with the only required field being the index information.
 *
 * The index information must be one of the following:
 *  - A std::initializer_list of varitensor::VarianceQualifiedIndex
 *        e.g. Tensor{ {index1, COVARIANT},  {index2, CONTRAVARIANT} }
 *  - A std::initializer_list of varitensor::Index
 *        e.g. Tensor{ index1, index2 }
 *  - A vector of varitensor::VarianceQualifiedIndex
 *        e.g. Tensor{ variance_qualified_index_vector }
 *  - A vector of varitensor::Index
 *        e.g. Tensor{ index_vector }
 *
 * The following example exercises the full functionality to create a two-dimensional tensor of ones called "T":
 *
 * @code{.cpp}
 * auto tensor = Tensor{
 *     "T",
 *     {
 *         {index1, COVARIANT},
 *         {index2, CONTRAVARIANT}
 *     },
 *     1
 * };
 * @endcode
 */
///@{
    // std::initializer_list<VarianceQualifiedIndex>
    Tensor(std::initializer_list<VarianceQualifiedIndex> vq_indices);
    Tensor(std::initializer_list<VarianceQualifiedIndex> vq_indices, double initial_value);
    Tensor(std::string name, std::initializer_list<VarianceQualifiedIndex> vq_indices);
    Tensor(std::string name, std::initializer_list<VarianceQualifiedIndex> vq_indices, double initial_value);

    // std::initializer_list<Index>
    Tensor(std::initializer_list<Index> indices);
    Tensor(std::initializer_list<Index> indices, double initial_value);
    Tensor(std::string name, std::initializer_list<Index> indices);
    Tensor(std::string name, std::initializer_list<Index> indices, double initial_value);

    // std::vector<VarianceQualifiedIndex>&
    explicit Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices);
    Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices, double initial_value);
    Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices);
    Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices, double initial_value);

    // std::vector<Index>&
    explicit Tensor(const std::vector<Index>& indices);
    Tensor(const std::vector<Index>& indices, double initial_value);
    Tensor(std::string name, const std::vector<Index>& indices);
    Tensor(std::string name, const std::vector<Index>& indices, double initial_value);
///@}

/**
 * @brief Initialise a scalar
 */
///@{
    explicit Tensor(double initial_value);
    Tensor(std::string name, double initial_value);
///@}

    template<typename E>
    requires impl::Expression_c<std::remove_cvref_t<E>> // "requires impl::Expression_c" to keep the forwarding reference
    Tensor(E&& expression) { // NOLINT - we want implicit conversion
        expression.populate(*this);

        if constexpr (!std::is_same_v<std::decay_t<E>, impl::LinkedOp>) {
            // for a linked operation, the widths are already correct
            size_t width = 1;
            for (auto& dimension: m_dimensions) {
                dimension.width = width;
                width *= dimension.size();
            }
        }
    }

    ~Tensor() noexcept = default;

    Tensor(const Tensor& other);
    Tensor& operator=(const Tensor& other);

    Tensor(Tensor&& other) noexcept;
    Tensor& operator=(Tensor&& other) noexcept;

/**
 * @brief Convert a scalar tensor to a double; throws if the tensor is not scalar.
 */
    explicit operator double() const;

/**
 * @brief Get a view on a slice of a tensor
 *
 * This function can generically slice a tensor with any combination of integers, Indices, and Intervals. Repeated
 * indices are summed over, according to the Einstein summation convention.
 *
 * @return An iterable, Tensor-convertible View object representing the slice.
 */
    template<impl::Indexable_c... Indices>
    requires (!impl::AllInt_c<Indices...>)
    [[nodiscard]] View operator[](Indices... indices) const {
        impl::deny(sizeof...(indices) != m_dimensions.size(),
                        "Indexing dimension mismatch");

        size_t n{0};
        size_t offset{0};
        impl::Dimensions passed_indices;
        construct_passed_indices(n, offset, passed_indices, indices...);

        return View{*this, m_data.get() + offset, passed_indices};
    }

/**
 * @brief Get a reference to a value in a tensor
 *
 * Special case of operator[] where all indices must be int.
 *
 * @return double& to a single value in the tensor.
 */
    template<impl::Indexable_c... Indices>
    requires (impl::AllInt_c<Indices...> && sizeof...(Indices) > 0)
    double& operator[](Indices... indices) {
        auto data = m_data.get();

        size_t n = 0;
        for(const auto index : {indices...}) {
            impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch");
            impl::deny(index < 0, "Indices cannot be less than zero");
            impl::deny(index >= m_dimensions[n].index.size(), "Index size mismatch");

            data += m_dimensions[n].width * index;
            ++n;
        }

        return *data;
    }

/**
 * @overload double& operator[](Indices... indices)
 */
    template<impl::Indexable_c... Indices>
    double& operator[](Indices...) requires (sizeof...(Indices) == 0) {
        return *m_data;
    }

/**
 * @brief Index a view with a vector of indexables
 */
///@{
    View operator[](impl::Indexables indices) const;
    double& operator[](const std::vector<int>& indices) const;
///@}

    [[nodiscard]] std::string name() const;
    [[nodiscard]] size_t size() const;
    [[nodiscard]] size_t size(int dimension) const;
    [[nodiscard]] int rank() const;
    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;
    [[nodiscard]] bool is_metric() const;
    [[nodiscard]] bool has_index(const Index& index) const;
    [[nodiscard]] size_t index_position(const Index& index) const;
    [[nodiscard]] std::vector<Index> indices() const;
    [[nodiscard]] const Index& indices(int index) const;
    [[nodiscard]] std::vector<VarianceQualifiedIndex> qualified_indices() const;
    [[nodiscard]] Variance variance(const Index& index) const;
    [[nodiscard]] Variance variance(int index) const;

/**
 * @brief Outputs a comma-separated dump of every value in the tensor
 */
    template<typename S>
    requires requires(S stream) {
        stream << std::string{""};
    }
    std::ostream& dump(S& ostream) const {
        // outputs a comma-separated dump of every value in the tensor
        const double* data = m_data.get();
        ostream << std::to_string(*data);
        for(size_t i=1; i<m_size; ++i) {
            ostream << std::string{", "} << std::to_string(*(data + i));
        }
        ostream << "\n";

        return ostream;
    }

/**
 * @brief Print the values of a tensor
 *
 * The tensor's values are displayed in a geometrically intuitive fashion up to rank 4. For higher rank tensors, the
 * values are just dumped as a list.
 */
    friend void write_data(std::ostream& stream, const Tensor& tensor);

/**
 * @brief Print a summary display of a tensor including name and indices
 *
 * Name and indices are displayed as a large title with the indices placed according to variance. The tensor's values
 * are displayed in a geometrically intuitive fashion up to rank 4. For higher rank tensors, the values are just dumped
 * as a list.
 */
    friend std::ostream& pretty_print(std::ostream& ostream, const Tensor& tensor);

/**
 * @brief Get the tensor's internal name (default to "Vari-Tensor")
 */
    Tensor& set_name(const std::string& name);
    /**
 * @brief Mathematically swap two indices and transpose the associated elements in memory
 */
    Tensor& transpose(const Index& first, const Index& second);
/**
 * @brief Replace one index with another
 */
    Tensor& relabel(const Index& old_index, const Index& new_index);
/**
 * @brief Directly set the variance of an index
 *
 * Does not manipulate the values of the tensor - for a true mathematical variance change, a tensor must be contracted
 * by multiplying with a metric tensor obtained from varitensor::metric_tensor(...).
 */
    Tensor& set_variance(const Index& index, Variance variance);
/**
 * @brief Alias for set_variance() with CONTRAVARIANT
 */
    Tensor& raise(const Index& index);
/**
 * @brief Alias for set_variance() with COVARIANT
 */
    Tensor& lower(const Index& index);

/**
 * @brief Standard iterator and const iterator functions
 */
///@{
    [[nodiscard]] View::iterator begin() const;
    [[nodiscard]] View::iterator end() const;
    [[nodiscard]] View::const_iterator cbegin() const;
    [[nodiscard]] View::const_iterator cend() const;
///@}

    impl::ProductOp operator-() const;

    friend impl::LinkedOp operator+(const Tensor& first, const Tensor& second);
    friend impl::LinkedOp operator+(const Tensor& first, const double& second);
    friend impl::LinkedOp operator+(const double& first, const Tensor& second);

    friend Tensor& operator+=(Tensor& first, const Tensor& second);
    friend double& operator+=(double& first, const Tensor& second);
    friend Tensor& operator+=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator+=(Tensor& first, Expression&& second) {
        (first + second).populate(first, false);
        return first;
    }

    friend impl::LinkedOp operator-(const Tensor& first, const Tensor& second);
    friend impl::LinkedOp operator-(const Tensor& first, const double& second);
    friend impl::LinkedOp operator-(const double& first, const Tensor& second);

    friend Tensor& operator-=(Tensor& first, const Tensor& second);
    friend double& operator-=(double& first, const Tensor& second);
    friend Tensor& operator-=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator-=(Tensor& first, Expression&& expression) {
        (first - expression).populate(first, false);
        return first;
    }

    friend impl::ProductOp operator*(const Tensor& first, const Tensor& second);
    friend impl::ProductOp operator*(const Tensor& first, const double& second);
    friend impl::ProductOp operator*(const double& first, const Tensor& second);

    friend Tensor& operator*=(Tensor& first, const Tensor& second);
    friend double& operator*=(double& first, const Tensor& second);
    friend Tensor& operator*=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator*=(Tensor& first, Expression&& second) {
        (first * second).populate(first);
        return first;
    }

    friend impl::ProductOp operator/(const Tensor& first, const Tensor& second);
    friend impl::ProductOp operator/(const Tensor& first, const double& second);
    friend impl::ProductOp operator/(const double& first, const Tensor& second);

    friend Tensor& operator/=(Tensor& first, const Tensor& second);
    friend double& operator/=(double& first, const Tensor& second);
    friend Tensor& operator/=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator/=(Tensor& first, Expression&& second) {
        (first / second).populate(first);
        return first;
    }

/**
 * @brief Check for equality
 *
 * Note that for tensor-tensor the comparison ignores name and whether or not the tensor is a metric tensor.
 */
///@{
    friend bool operator==(const Tensor& first, const Tensor& second);
    friend bool operator==(const Tensor& first, const double& second);
    friend bool operator==(const double& first, const Tensor& second);
///@}

private:
    friend class View;
    friend class impl::LinkedOp;
    friend class impl::ProductOp;

    impl::Dimensions m_dimensions;
    size_t m_size{1};
    impl::DoublePtr m_data{nullptr, [](double*){}};
    std::string m_name{impl::TENSOR_DEFAULT_NAME};
    impl::TensorClass m_tensor_class{impl::TENSOR};

    friend Tensor levi_civita_symbol(std::initializer_list<Index> indices);
    friend Tensor levi_civita_symbol(std::initializer_list<VarianceQualifiedIndex> indices);
    friend Tensor antisymmetric_symbol(std::initializer_list<Index> indices);
    friend Tensor antisymmetric_symbol(std::initializer_list<VarianceQualifiedIndex> indices);
    friend Tensor kronecker_delta(std::initializer_list<Index> indices);
    friend Tensor kronecker_delta(std::initializer_list<VarianceQualifiedIndex> indices);
    friend Tensor metric_tensor(std::initializer_list<VarianceQualifiedIndex>, const MetricFunction&);
    // unqualified indices don't make sense for a metric tensor

    template<impl::Indices_c I>
    static Tensor make_levi_civita_symbol(const I& indices) {
        impl::deny(indices.size() <= 1,
                        "Levi-Civita Symbol must have at least 2 indices");

        const auto expected_size = indices.begin()->index.size();
        for (const auto& [index, _]: indices) {
            impl::deny(index.size() != expected_size,
                            "Indices to Levi-Civita Symbol must all the the same size");
        }

        auto epsilon = Tensor{"Epsilon",  indices};
        epsilon.set_tensor_class(impl::LEVI_CIVITA_SYMBOL);

        std::vector<int> permutation(indices.size());
        std::ranges::iota(permutation, 0);
        do {
            int inversions = 0;
            for (size_t i=0; i<permutation.size(); ++i) {
                for (size_t j=0; j<permutation.size(); ++j) {
                    if (
                        (j < i && permutation[i] < permutation[j]) ||
                        (j > i && permutation[i] > permutation[j])
                    ) {
                        ++inversions;
                    }
                }
            }
            if (inversions / 2 % 2) epsilon[permutation] = -1;
            else epsilon[permutation] = 1;
        }
        while (std::ranges::next_permutation(permutation).found);

        return epsilon;
    }

    template<impl::Indices_c I>
    static Tensor make_kronecker_delta(const I& indices) {
        impl::deny(indices.size() <= 1,
                        "Kronecker Delta must have at least 2 indices");

        const auto expected_size = indices.begin()->index.size();
        for (const auto& [index, _]: indices) {
            impl::deny(index.size() != expected_size,
                            "Indices to Kronecker Delta must all the the same size");
        }

        auto delta = Tensor{"delta", indices};
        delta.set_tensor_class(impl::KRONECKER_DELTA);

        // The diagonal memory locations are multiples of the sum of the index widths; for a
        // contiguous, symmetric tensor, this sum is given by the geometric series:
        // sum[n=0 to rank](index_length^n).
        const auto index_length_sum = static_cast<size_t>(
            (std::pow(expected_size, delta.rank()) - 1) / (expected_size - 1)
        );

        double* data = delta.m_data.get();
        for (int i=0; i<expected_size; ++i) {
            data[i * index_length_sum] = 1;
        }

        return delta;
    }

    void set_tensor_class(impl::TensorClass tensor_class);

    template<impl::VarianceQualifiedIndices_c V>
    void from_variance_qualified_indices(const V& variance_qualified_indices) {
        for(auto& vqi: variance_qualified_indices) {
            for (auto& dimension: m_dimensions) {
                impl::deny(dimension.index.id() == vqi.index.id(),
                                "Cannot initialize tensor with repeated indices");
            }
            m_dimensions.emplace_back(vqi.index, vqi.variance, m_size);
            m_size *= vqi.index.size();
        }
    }

    template<typename IteratorType>
    void populate_general(IteratorType& iter, IteratorType end, const bool allocate) {
        if (allocate) {
            const size_t new_size = iter.size();
            impl::DoublePtr new_data = impl::allocate(new_size);

            double* running_ptr = new_data.get();
            for (; iter != end; ++iter) *running_ptr++ = iter.deref();

            m_dimensions = iter.dimensions();
            m_size = new_size;
            m_data.swap(new_data);
        }
        else {
            double* running_ptr = m_data.get();
            for (; iter != end; ++iter) *running_ptr++ = iter.deref();
        }
    }

    void populate_scalar(double scalar, bool allocate);

    template<impl::Indexable_c... Indices>
    void construct_passed_indices(
        size_t& n,
        size_t& offset,
        impl::Dimensions& passed_indices,
        const Index& index,
        Indices... indices
    ) const {
        impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch!");
        impl::deny(index.size() > m_dimensions[n].index.size(), "Index size mismatch");

        passed_indices.emplace_back(index, m_dimensions[n].variance, m_dimensions[n].width);
        construct_passed_indices(++n, offset, passed_indices, indices...);
    }

    template<impl::Indexable_c... Indices>
    void construct_passed_indices(
        size_t& n,
        size_t& offset,
        impl::Dimensions& passed_indices,
        const Interval& index,
        Indices... indices
    ) const {
        impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch!");
        impl::deny(index.last >= m_dimensions[n].index.size(), "Index size mismatch");

        offset += m_dimensions[n].width * index.first;
        passed_indices.emplace_back(index, m_dimensions[n].variance, m_dimensions[n].width);
        construct_passed_indices(++n, offset, passed_indices, indices...);
    }

    template<impl::Indexable_c... Indices>
    void construct_passed_indices(
        size_t& n,
        size_t& offset,
        impl::Dimensions& passed_indices,
        const int index,
        Indices... indices
    ) const {
        impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch!");
        impl::deny(index+1 > m_dimensions[n].index.size(), "Index size mismatch");

        offset += index * m_dimensions[n].width;
        construct_passed_indices(++n, offset, passed_indices, indices...);
    }

    static void construct_passed_indices(size_t&, size_t&, impl::Dimensions&) {} // terminating overload
};

namespace impl {

template<>
inline bool ViewIterator<>::is_metric() const {
    return m_target->is_metric();
}

template<>
inline bool ViewIterator<true>::is_metric() const {
    return m_target->is_metric();
}

struct GetTensor{
    template <typename E>
    Tensor operator()(E&& expression) {
        return Tensor{expression};
    }
};

#if defined (__AVX__)

#include <immintrin.h>

constexpr size_t REG_WIDTH_256 = 4;
constexpr std::align_val_t M256_ALIGN{alignof(__m256d)};

inline void deallocate(double* data) {
    operator delete[](data, M256_ALIGN);
}

inline DoublePtr allocate(const size_t size) {
    const size_t remainder = size % REG_WIDTH_256;
    const size_t padded_size = remainder ? size - remainder + REG_WIDTH_256 : size;

    return DoublePtr{
        static_cast<double*> (
            operator new[](sizeof(double) * padded_size, M256_ALIGN)
        ),
        deallocate
    };
}

inline DoublePtr allocate_zeroed(const size_t size) {
    DoublePtr data = allocate(size);
    std::memset(data.get(), 0, size*sizeof(double));
    return data;
}

inline void copy(double* data1, const double* data2, const size_t size) {
    size_t i{0};
    for (; i<size - size%REG_WIDTH_256; i+=REG_WIDTH_256) {
        _mm256_store_pd(
            data1 + i,
            _mm256_loadu_pd(data2 + i)
        );
    }
    for (; i<size; ++i) data1[i] = data2[i];
}

inline void broadcast_vec(const double* data1, double* data2, const size_t size1, const size_t size2) {
    size_t j_start{0};
    for (int i=0; i < static_cast<int>(size1 - REG_WIDTH_256 + 1); i += REG_WIDTH_256) {
        const __m256d values = _mm256_load_pd(data1 + i);
        for (size_t j=j_start; j<size2; j+=size1) {
            _mm256_storeu_pd(data2 + j, values);
        }
        j_start += REG_WIDTH_256;
    }

    for (size_t i=size1 - size1 % REG_WIDTH_256; i<size1; ++i) {
        const double value = data1[i];
        for (size_t j = j_start; j<size2; j+=size1) {
            data2[j] = value;
        }
        ++j_start;
    }
}

inline void broadcast_chunks(const double* data1, double* data2, const size_t size1, const size_t interval) {
    double* running_ptr = data2;
    for (size_t i=0; i < size1; ++i) {
        const __m256d values = _mm256_set1_pd(data1[i]);
        for (int j=0; j < static_cast<int>(interval - REG_WIDTH_256 + 1); j += REG_WIDTH_256) {
            _mm256_storeu_pd(
                running_ptr,
                _mm256_mul_pd(
                    values,
                    _mm256_loadu_pd(running_ptr)
                )
            );

            running_ptr += REG_WIDTH_256;
        }
        for (size_t j=0; j < interval % REG_WIDTH_256; ++j) {
            *running_ptr *= data1[i];
            ++running_ptr;
        }
    }
}

inline void broadcast(const double value, double* data, const size_t size) {
    const __m256d scalar_vec = _mm256_set1_pd(value);
    for (size_t i=0; i<size; i+=REG_WIDTH_256) {
        _mm256_store_pd(
            data + i,
            _mm256_mul_pd(
                _mm256_load_pd(data + i),
                scalar_vec
            )
        );
    }
}

inline void piecewise(const double* data1, const double* data2,  double* data3, const size_t size, const Operation operation) {
    switch (operation) {
    case ADD:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_add_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    case SUB:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_sub_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    case MUL:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_mul_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    case DIV:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_div_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    }
}

#else

inline void deallocate(double* data) {
    std::free(data);
}

inline DoublePtr allocate(const size_t size) {
    auto data = DoublePtr{
        static_cast<double*>(malloc(size * sizeof(double))),
        deallocate
    };
    if (!data) throw std::bad_alloc{};
    return data;
}

inline DoublePtr allocate_zeroed(const size_t size) {
    return DoublePtr{
        static_cast<double*>(calloc(size, sizeof(double))),
        deallocate
    };
}

inline void copy(double* data1, const double* data2, const size_t size) {
    for (size_t i=0; i<size; ++i) data1[i] = data2[i];
}

inline void broadcast_vec(const double* data1, double* data2, const size_t size1, const size_t size2) {
    for (size_t i=0; i < size1; i++) {
        const double current_value = data1[i];
        for (size_t j=i; j < size2; j += size1) {
            data2[j] = current_value;
        }
    }
}

inline void broadcast_chunks(const double* data1, double* data2, const size_t size1, const size_t interval) {
    for (size_t i=0; i < size1; ++i) {
        const double current_value = data1[i];
        for (size_t j=0; j < interval; j++) {
            *data2 *= current_value;
            ++data2;
        }
    }
}

inline void broadcast(const double value, double* data, const size_t size) {
    for (size_t i=0; i<size; ++i) data[i] *= value;
}

inline void piecewise(const double* data1, const double* data2,  double* data3, const size_t size, const Operation operation) {
    switch (operation) {
    case ADD: for (size_t i=0; i<size; ++i) data3[i] = data1[i] + data2[i]; break;
    case SUB: for (size_t i=0; i<size; ++i) data3[i] = data1[i] - data2[i]; break;
    case MUL: for (size_t i=0; i<size; ++i) data3[i] = data1[i] * data2[i]; break;
    case DIV: for (size_t i=0; i<size; ++i) data3[i] = data1[i] / data2[i]; break;
    }
}

#endif

inline DoublePtr allocate(const size_t size, const double& initial_value) {
    DoublePtr data = allocate(size);
    std::fill_n(data.get(), size, initial_value);
    return data;
}

inline DoublePtr allocate_copy(const double* data, const size_t size) {
    DoublePtr copy = allocate(size);
    std::memcpy(copy.get(), data, size * sizeof(double));
    return copy;
}

struct VariTensorInternalError final: std::logic_error {
    explicit VariTensorInternalError(const std::string& message):
        std::logic_error("<Vari-Tensor> Tensor logic error: " + message)
    {}
};

/**
 * @brief Increments the positions vector in accordance with the dimensions vector
 *
 * The iterator provided will be told which index to increment, or reset if a dimension is overflown. Returns true if
 * the increment was successful, false if the end of the dimensions has been reached.
 */
template <ExpressionIterator_c E>
bool increment_positions( // static to avoid having to use virtual functions
    std::vector<int>& positions,
    const std::vector<Dimension>& dimensions,
    const E& iterator // not really const, but we have to pretend to maintain the constness of the * operators
) {
    for (size_t i=0; i<positions.size(); ++i) {
        // check if we're about to overflow the next index
        if (positions[i] + 1 == dimensions[i].index.size()) [[unlikely]] {
            // if so, reset it and move on to the next one
            positions[i] = 0;
            iterator.reset(dimensions[i].index.id());
            continue;
        }

        // once we've found an index we can increment, do so
        ++positions[i];
        iterator.increment(dimensions[i].index.id());

        return true;
    }

    // note that if we reach the end, all the indices will have been reset to 0
    return false;
}

} // namespace impl

namespace impl {

class LinkedOpIterator: public ExpressionIteratorBase {
public:
    LinkedOpIterator() = default;
    LinkedOpIterator(double modifier, Preparatory& preparatory, const std::vector<Operation>* signs, bool end=false);
    LinkedOpIterator(const LinkedOpIterator& other) = default;
    LinkedOpIterator& operator=(const LinkedOpIterator& other) = default;
    LinkedOpIterator(LinkedOpIterator&& other) noexcept = default;
    LinkedOpIterator& operator=(LinkedOpIterator&& other) = default;

    LinkedOpIterator& operator++();
    LinkedOpIterator operator++(int);

    double operator*() const;

    bool operator==(const LinkedOpIterator& other) const;

    [[nodiscard]] static bool is_metric();
    [[nodiscard]] static bool is_contiguous();
    [[nodiscard]] static double* data();

    void increment(int index_id) const;
    void reset(int index_id) const;

    [[nodiscard]] double deref() const;

private:
    double m_modifier{0};
    ExpressionIterators m_sub_iterators{};
    const std::vector<Operation>* m_signs{nullptr};  // * to allow default construction

    bool m_end{false};
};

class ProductOpIterator: public ExpressionIteratorBase {
public:
    ProductOpIterator() = default;
    ProductOpIterator(double modifier, Preparatory& preparatory, bool end=false);
    ProductOpIterator(const ProductOpIterator& other) = default;
    ProductOpIterator& operator=(const ProductOpIterator& other) = default;
    ProductOpIterator(ProductOpIterator&& other) noexcept = default;
    ProductOpIterator& operator=(ProductOpIterator&& other) = default;

    ProductOpIterator& operator++();
    ProductOpIterator operator++(int);

    double operator*() const;

    bool operator==(const ProductOpIterator& other) const;

    [[nodiscard]] static bool is_metric();
    [[nodiscard]] static bool is_contiguous();
    [[nodiscard]] static double* data();

    void increment(const int index_id) const;
    void reset(const int index_id) const;

    [[nodiscard]] double deref() const;

private:
    double m_modifier{1};
    ExpressionIterators m_sub_iterators{};

    // for index summation
    Dimensions m_repeated;
    mutable std::vector<int> m_repeated_positions; // mutable for deref()

    bool m_end{false};
};

inline auto Deref = [](const auto& expression) {return expression.deref();};
inline auto IsContiguous = [](const auto& iter) {return iter.is_contiguous();};

class LinkedOp {
public:
    template <ExpressionOperand_c OperandType1, ExpressionOperand_c OperandType2>
    LinkedOp(const OperandType1& first, const OperandType2& second, Operation sign) {
        call_add_function(first);
        call_add_function(second, sign);
    }

    void populate(Tensor& tensor, bool allocate = true);

    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;

    using iterator = LinkedOpIterator;
    [[nodiscard]] iterator begin() const;
    [[nodiscard]] iterator end() const;
    [[nodiscard]] ExpressionIterator vbegin() const;

private:
    double m_modifier{0}; // used to collate double/scalar operands
    Expressions m_sub_expressions{};
    std::vector<Operation> m_signs;  // used to track the sign of each sub-expression

    template<ExpressionOperand_c OperandType>
    void call_add_function(const OperandType& operand, const Operation sign=ADD) {
        if constexpr (!std::is_same_v<OperandType, double>) {
            if (operand.is_scalar()) {
                // If we have a scalar Tensor or Expression, we can short-circuit by getting the value
                add_element(operand.get_scalar(), sign);
                return;
            }
        }
        add_element(operand, sign);
    }

    void add_element(double value, Operation sign);
    void add_element(const Tensor& tensor, Operation sign);
    void add_element(const LinkedOp& linked_op, Operation sign);
    void add_element(const ProductOp& product_op, Operation sign);

    void add_element(const View& view, Operation sign);
};

class ProductOp {
public:
    using iterator = ProductOpIterator;

    template <ExpressionOperand_c OperandType1, ExpressionOperand_c OperandType2>
    ProductOp(const OperandType1& first, const OperandType2& second, Operation operation)
    {
        // order matters - indices need to be added in the order they arrive
        call_add_function(first);
        call_add_function(second, operation);
    }

    void populate(Tensor& tensor);

    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;

    [[nodiscard]] iterator begin() const;
    [[nodiscard]] iterator end() const;
    [[nodiscard]] ExpressionIterator vbegin() const;

private:
    double m_modifier{1}; // used to collate double/scalar operands
    Expressions m_sub_expressions{};

    template<ExpressionOperand_c OperandType>
    void call_add_function(const OperandType& operand, const Operation operation=MUL) {
        if constexpr (!std::is_same_v<OperandType, double>) {
            if (operand.is_scalar()) {
                // If we have a scalar Tensor or Expression, we can short-circuit by getting the value
                add_element(operation == MUL ? operand.get_scalar() : 1/operand.get_scalar());
            }
            else {
                deny(operation == DIV, "Cannot divide by non-scalar Tensor/Expression");
                add_element(operand);
            }
        }
        else {
            add_element(operation == MUL ? operand : 1/operand);
        }
    }

    void add_element(const double& value);
    void add_element(const Tensor& tensor);
    void add_element(const LinkedOp& linked_op);
    void add_element(const ProductOp& product_op);
    void add_element(const View& view);
};

} // namespace impl

/**
 * @brief Basic Euclidean metric function, used as the default for metric_tensor(...)
 */
const MetricFunction EUCLIDEAN_METRIC = [](const int i, const int j) {
    return i == j ? 1 : 0;
};

/**
 * @brief Creates a metric tensor that can be used to raise and lower indices
 *
 * Metric tensors function like normal tensors, but when used in einstein summation, their non-summed index positionally
 * replaces the summed one in the new tensor. This is commonly used in differential geometry to raise and lower indices.
 *
 * e.g. B_i_j * g^i^k = B^k_j
 *
 * This tensor will always use a lowercase g symbol when pretty printing.
 *
 * @param indices The indices with which to initialise the metric. There must be two indices, both of the same length.
 *
 * @param metric_function A user-defined function to populate the metric. The function must accept two ints giving
 * the position in the metric and return a double giving the value; varitensor::MetricFunction is provided as a
 * helper alias.
 */
inline Tensor metric_tensor(
    const std::initializer_list<VarianceQualifiedIndex> indices,
    const MetricFunction& metric_function = EUCLIDEAN_METRIC
) {
    impl::deny(indices.size() != 2,
                    "Metric tensor must have exactly 2 indices");
    impl::deny(indices.begin()->index == (indices.begin() + 1)->index,
                    "Metric tensor indices must be different");
    impl::deny(indices.begin()->variance != (indices.begin() + 1)->variance,
                    "Metric tensor indices must have the same variance");

    Tensor metric{"g", indices};
    metric.set_tensor_class(impl::METRIC_TENSOR);

    const auto index_size = indices.begin()->index.size();
    for (int i = 0; i < index_size; ++i) {
        for (int j = 0; j < index_size; ++j) {
            metric[i, j] = metric_function(i, j);
        }
    }

    return metric;
}

/**
 * @brief Creates an n-dimensional Levi-Civita symbol
 *
 * This tensor will always use an epsilon symbol when pretty-printing.
 *
 * @param indices The indices with which to initialise the symbol. There must be at least 2 indices, and they must all
 * be the same length.
 */
///@{
inline Tensor levi_civita_symbol(const std::initializer_list<Index> indices) {
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    return Tensor::make_levi_civita_symbol(vq_indices);
}

inline Tensor levi_civita_symbol(const std::initializer_list<VarianceQualifiedIndex> indices) {
    return Tensor::make_levi_civita_symbol(indices);
}

inline Tensor antisymmetric_symbol(const std::initializer_list<Index> indices) {
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    return Tensor::make_levi_civita_symbol(vq_indices);
}

inline Tensor antisymmetric_symbol(const std::initializer_list<VarianceQualifiedIndex> indices) {
    return Tensor::make_levi_civita_symbol(indices);
}
///@}

/**
 * @brief Creates an n-dimensional Kronecker delta
 *
 * This tensor will always use an delta symbol when pretty-printing.
 *
 * @param indices The indices with which to initialise the delta. There must be at least 2 indices, and they must all
 * be the same length.
 */
///@{
inline Tensor kronecker_delta(const std::initializer_list<Index> indices) {
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    return Tensor::make_kronecker_delta(vq_indices);
}

inline Tensor kronecker_delta(const std::initializer_list<VarianceQualifiedIndex> indices) {
    return Tensor::make_kronecker_delta(indices);
}
///@}

// =================================================================================================
//                                                                                Preparatory impl |
// =================================================================================================

namespace impl {
inline Preparatory::Preparatory(const Expressions& sub_expressions, const PreparatoryType type) {
    if (sub_expressions.empty()) return; // scalar case

    if (type == LINKED) prepare_linked_operation(sub_expressions);
    else prepare_product_operation(sub_expressions);
}

inline void Preparatory::prepare_linked_operation(const Expressions& sub_expressions) {
    state = ALIGNED_INDICES;

    // construct the first iterator so that we have something to compare against
    sub_iterators.emplace_back(std::visit(VBegin, sub_expressions[0]));
    dimensions = std::visit(GetDimensions, sub_iterators[0]);
    size = std::visit(GetSize, sub_iterators[0]);

    std::function<void(unsigned&, const Dimensions&)> index_matching_function;

    auto find_index = [this] (
        const unsigned& j, const Dimensions& current_dimensions, unsigned k=0
    ) {
        for (; k<current_dimensions.size(); ++k) {
            if (dimensions[j].index == current_dimensions[k].index) {
                deny(dimensions[j].variance != current_dimensions[k].variance,
                    "Cannot add or subtract tensors with indices of disagreeing variance");
                return;
            }
        }
        deny(true, // this is always an error, but go through deny() for consistency
            "Cannot add or subtract tensors with un-pairable indices");
    };

    auto assume_aligned = [this, &find_index, &index_matching_function](
        const unsigned& j, const Dimensions& current_dimensions
    ) {
        deny(dimensions[j].variance != current_dimensions[j].variance,
                "Cannot add or subtract tensors with indices of disagreeing variance");
        if (dimensions[j].index != current_dimensions[j].index) {
            state = GENERAL;
            find_index(j, current_dimensions, j);
            index_matching_function = find_index; // switch to the general case
        }
    };

    // assume that we have only views with aligned indices, e.g. (T_ab + U_ab) NOT (T_ab + U_ba)
    index_matching_function = assume_aligned;
    for (unsigned i=1; i<sub_expressions.size(); ++i) { // start i at 1 as we just did the first iterator
        sub_iterators.emplace_back(std::visit(VBegin, sub_expressions[i]));
        deny(std::visit(GetDimensions, sub_iterators[i]).size() != dimensions.size(),
            "Cannot add or subtract tensors with different numbers of indices");

        const Dimensions& current_dimensions = std::visit(GetDimensions, sub_iterators[i]);
        for (unsigned j=0; j<dimensions.size(); ++j) {
            index_matching_function(j, current_dimensions);
        }
    }
}

inline void Preparatory::prepare_product_operation(const Expressions& sub_expressions) {
    state = FREE_MULTIPLICATION;

    for (auto& expression: sub_expressions) sub_iterators.emplace_back(std::visit(VBegin, expression));

    // get the naive dimensions whilst recording metric and repetition information that will be useful later
    Dimensions total_dimensions;
    std::map<int, Couple> partners;
    int i{0};

    for (auto& iterator: sub_iterators) {
        auto sub_dimensions = std::visit(GetDimensions, iterator);

        for (size_t k = 0; k < sub_dimensions.size(); ++k) {
            total_dimensions.emplace_back(sub_dimensions[k]);

            if (std::visit([](auto& expression) {return expression.is_metric();}, iterator)) {
                partners[sub_dimensions[k].index.id()].add(i, true);
                const int other = k == 0 ? i+1 : i-1; // metric tensors only have 2 indices
                partners[sub_dimensions[k].index.id()].metric_index = other;
            }
            else {
                partners[sub_dimensions[k].index.id()].add(i, false);
            }

            ++i;
        }
    }

    // handle any metric indices
    for (auto& couple: partners | std::views::values) {
        if (couple.is_repeated() && couple.is_metric()) {
            std::swap(total_dimensions[couple.metric_index], total_dimensions[couple.other_index]);
        }
    }

    for (auto& dimension: total_dimensions) {
        if (auto& couple = partners[dimension.index.id()]; couple.is_repeated()) { // repeated index
            repeated.emplace_back(dimension);
            couple.clear();
            state = GENERAL;
        }
        else if (couple.has()) { // non-repeated index
            dimensions.push_back(dimension);
            dimensions.back().width = size;
            size *= dimension.size();
        }
    }
}

} // namespace impl

// =================================================================================================
//                                                                                       View impl |
// =================================================================================================

inline View::View(const Tensor& target):
    m_target{target},
    m_data_ptr{target.m_data.get()},
    m_dimensions{m_target.m_dimensions}
{
    for (size_t i = 0; i < m_dimensions.size(); ++i) {
        impl::deny(m_dimensions[i].size() > m_target.m_dimensions[i].index.size(),
                        "Index size mismatch");
        impl::deny(m_dimensions[i].variance != m_target.m_dimensions[i].variance,
                        "Index variance mismatch");
    }
}

inline View::View(const Tensor& target, double* const data_ptr, impl::Dimensions dimensions):
    m_target{target},
    m_data_ptr{data_ptr},
    m_dimensions{std::move(dimensions)}
{} // we only call this from Tensor::operator[], so no need to validate the dimensions

void View::operator=(const Tensor& other) && { // NOLINT - allows T[i, 2] = U
    const View other_view{other};

    impl::deny(m_dimensions.size() != other.m_dimensions.size(),
                    "Cannot assign to View with different dimensions");
    for (size_t i = 0; i < m_dimensions.size(); ++i) {
        impl::deny(m_dimensions[i].index.size() != other.m_dimensions[i].index.size(),
                        "Index size mismatch when assigning view");
        impl::deny(m_dimensions[i].variance != other.m_dimensions[i].variance,
                        "Variance mismatch when assigning view");
    }

    auto iter1 = begin();
    auto iter2 = other_view.begin();
    impl::deny(iter1.is_contracted(), "Cannot assign to a contraction expression");

    for (auto end=this->end(); iter1 != end; ++iter1, ++iter2) *iter1 = *iter2;
}

inline bool View::operator==(const View& other) const {
    auto iter1 = begin();
    auto iter2 = other.begin();
    for (auto end = this->end(); iter1 != end;  ++iter1, ++iter2) {
        if (*iter1 != *iter2) return false;
    }
    return iter2.finished();
}

inline View::iterator View::begin() {
    return iterator{&m_target, m_data_ptr, m_dimensions};
}

inline View::iterator View::end() {
    return iterator{&m_target, nullptr, {}};
}

inline View::const_iterator View::begin() const {
    return const_iterator{&m_target, m_data_ptr, m_dimensions};
}

inline View::const_iterator View::end() const {
    return const_iterator{&m_target, nullptr, {}};
}

inline View::const_iterator View::cbegin() const {
    return const_iterator{&m_target, m_data_ptr, m_dimensions};
}

inline View::const_iterator View::cend() const {
    return const_iterator{&m_target, nullptr, {}};
}

inline impl::ExpressionIterator View::vbegin() const {
    return begin();
}

[[nodiscard]] inline double* View::data() const {
    return m_data_ptr;
}

[[nodiscard]] inline bool View::is_scalar() const {
    return m_dimensions.empty();
}

[[nodiscard]] inline double View::get_scalar() const {
    return *m_data_ptr;
}

inline void View::populate(Tensor& tensor, const bool allocate/* = true */) const {
    auto iter = cbegin();

    if (allocate) {
        tensor.m_dimensions = iter.dimensions();
        tensor.m_size = iter.size();
        tensor.m_data = impl::allocate(tensor.m_size);
    }

    if (!iter.is_contracted() && iter.is_contiguous()) {
        impl::copy(tensor.m_data.get(), m_data_ptr, tensor.m_size);
    }
    else {
        double* running_ptr = tensor.m_data.get();
        for (const auto end=cend(); iter != end; ++iter, ++running_ptr) {
            *running_ptr = *iter;
        }
    }
}

// =================================================================================================
//                                                                           LinkedOpIterator impl |
// =================================================================================================
namespace impl {

[[nodiscard]] inline double* LinkedOpIterator::data() {
    throw VariTensorInternalError("Function should never be called");
}

inline LinkedOpIterator::LinkedOpIterator(
    const double modifier,
    Preparatory& preparatory,
    const std::vector<Operation>* signs,
    const bool end/* = false */
):
    ExpressionIteratorBase{preparatory},
    m_modifier{modifier},
    m_sub_iterators{std::move(preparatory.sub_iterators)},
    m_signs{signs},
    m_end{end}
{}

inline LinkedOpIterator& LinkedOpIterator::operator++() {
    m_end = !increment_positions(m_positions, m_dimensions, *this);
    return *this;
}

inline LinkedOpIterator LinkedOpIterator::operator++(int) {
    LinkedOpIterator copy = *this;
    ++*this;
    return copy;
}

inline double LinkedOpIterator::operator*() const {
    return deref();
}

inline bool LinkedOpIterator::operator==(const LinkedOpIterator& other) const {
    if (m_end != other.m_end) [[likely]] return false;
    if (m_end) return true;
    return m_modifier == other.m_modifier && m_positions == other.m_positions; // good enough for normal iteration
}

inline void LinkedOpIterator::increment(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Increment(index_id), iterator);
}

inline void LinkedOpIterator::reset(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Reset(index_id), iterator);
}

inline double LinkedOpIterator::deref() const {
    double sum = 0;
    for (size_t i=0; i < m_sub_iterators.size(); ++i) {
        sum += std::visit(Deref, m_sub_iterators[i]) * static_cast<double> ((*m_signs)[i]);
    }
    return sum + m_modifier;
}

inline bool LinkedOpIterator::is_metric() {
    return false;
}

inline bool LinkedOpIterator::is_contiguous() {
    return false;
}

// =================================================================================================
//                                                                          ProductOpIterator impl |
// =================================================================================================

[[nodiscard]] inline double* ProductOpIterator::data() {
    throw VariTensorInternalError("Function should never be called");
}

inline ProductOpIterator::ProductOpIterator(
    const double modifier,
    Preparatory& preparatory,
    const bool end/* = false */
):
    ExpressionIteratorBase{preparatory},
    m_modifier{modifier},
    m_sub_iterators{std::move(preparatory.sub_iterators)},
    m_repeated{std::move(preparatory.repeated)},
    m_repeated_positions(m_repeated.size(), 0),
    m_end{end}
{}

inline ProductOpIterator& ProductOpIterator::operator++() {
    m_end = !increment_positions(m_positions, m_dimensions, *this);
    return *this;
}

inline ProductOpIterator ProductOpIterator::operator++(int) {
    ProductOpIterator copy = *this;
    ++*this;
    return copy;
}

inline double ProductOpIterator::operator*() const {
    return deref();
}

inline bool ProductOpIterator::operator==(const ProductOpIterator& other) const {
    if (m_end != other.m_end) [[likely]] return false;
    if (m_end) return true;
    return m_modifier == other.m_modifier && m_positions == other.m_positions; // good enough for normal iteration
}

inline void ProductOpIterator::increment(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Increment(index_id), iterator);
}

inline void ProductOpIterator::reset(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Reset(index_id), iterator);
}

inline double ProductOpIterator::deref() const {
    /* Despite modifying m_repeated_positions, this function is const as it always returns the positions to 0 */
    double sum = 0;

    do {
        double product = 1;
        for (auto& iterator: m_sub_iterators) product *= std::visit(Deref, iterator);
        sum += product;
    }
    while (increment_positions(m_repeated_positions, m_repeated, *this));

    return sum * m_modifier;
}

inline bool ProductOpIterator::is_metric() {
    return false;
}

inline bool ProductOpIterator::is_contiguous() {
    return false;
}

// =================================================================================================
//                                                                                   LinkedOp impl |
// =================================================================================================

inline LinkedOpIterator LinkedOp::begin() const {
    Preparatory preparatory{m_sub_expressions, LINKED};
    return iterator{m_modifier, preparatory, &m_signs};
}

inline LinkedOpIterator LinkedOp::end() const {
    Preparatory preparatory{};
    return iterator{m_modifier, preparatory, &m_signs, true};
}

inline ExpressionIterator LinkedOp::vbegin() const {
    return begin();
}

[[nodiscard]] inline bool LinkedOp::is_scalar() const {
    return m_sub_expressions.empty();
}

[[nodiscard]] inline double LinkedOp::get_scalar() const {
    return m_modifier;
}

inline void LinkedOp::add_element(const double value, const Operation sign) {
    deny(!is_scalar(), "Cannot add/subtract scalar with non-scalar expression");
    sign == ADD ? m_modifier += value : m_modifier -= value;
}

inline void LinkedOp::add_element(const Tensor& tensor, const Operation sign) {
    m_sub_expressions.emplace_back(View{tensor});
    sign == ADD ? m_signs.push_back(ADD) : m_signs.push_back(SUB);
}

inline void LinkedOp::add_element(const LinkedOp& linked_op, const Operation sign) {
    if (sign == ADD) {
        m_modifier += linked_op.m_modifier;
        m_signs.append_range(linked_op.m_signs);
    }
    else {
        m_modifier -= linked_op.m_modifier;
        for (auto& sub_sign: linked_op.m_signs) m_signs.push_back(static_cast<Operation>(sub_sign * -1));
    }
    m_sub_expressions.append_range(linked_op.m_sub_expressions);
}

inline void LinkedOp::add_element(const ProductOp& product_op, const Operation sign) {
    m_sub_expressions.emplace_back(product_op);
    m_signs.push_back(sign);
}

inline void LinkedOp::add_element(const View& view, const Operation sign) {
    m_sub_expressions.emplace_back(view);
    m_signs.push_back(sign);
}

inline void LinkedOp::populate(Tensor& tensor, const bool allocate/* = true */) {
    auto populate_aligned_indices = [&](Preparatory& preparatory) {
        if (allocate) {
            tensor.m_dimensions = preparatory.dimensions;
            tensor.m_size = preparatory.size;
            DoublePtr new_data = allocate_copy(
                std::visit(IsContiguous, preparatory.sub_iterators[0]) ?
                    std::get<View>(m_sub_expressions[0]).data() :
                    std::visit(GetTensor{}, m_sub_expressions[0]).m_data.get(),
                tensor.m_size
            );
            tensor.m_data.swap(new_data);
        }

        // for +=/-=, we start with the 0th in the right place, for +/- we handled with 0th above, so start i at 1
        for (unsigned i=1; i<preparatory.sub_iterators.size(); ++i) {
            piecewise(
                tensor.m_data.get(),
                std::visit(IsContiguous, preparatory.sub_iterators[i]) ?
                    std::get<View>(m_sub_expressions[i]).data() :
                    std::visit(GetTensor{}, m_sub_expressions[i]).m_data.get(),
                tensor.m_data.get(),
                tensor.m_size,
                m_signs[i]
            );
        }
    };

    switch (Preparatory preparatory{m_sub_expressions, LINKED}; preparatory.state) {
    case SCALAR:
        tensor.populate_scalar(m_modifier, allocate);
        break;
    case ALIGNED_INDICES:
        populate_aligned_indices(preparatory);
        break;
    case GENERAL: {
        iterator iter{m_modifier, preparatory, &m_signs};
        tensor.populate_general(iter, end(), allocate);
        break;
    }
    case FREE_MULTIPLICATION: // to stop the compiler warning
    default:
        throw VariTensorInternalError("Invalid expression state!");
    }
}

// =================================================================================================
//                                                                                  ProductOp impl |
// =================================================================================================

inline void ProductOp::populate(Tensor& tensor) {
    auto populate_free_multiplication = [&](Preparatory& preparatory) {
        DoublePtr new_data = allocate(preparatory.size, 0);

        if (std::visit(IsContiguous, preparatory.sub_iterators[0])) {
            broadcast_vec(
                std::visit(GetData, preparatory.sub_iterators[0]),
                new_data.get(),
                std::visit(GetSize, preparatory.sub_iterators[0]),
                preparatory.size
            );
        }
        else {
            const auto tmp = std::visit(GetTensor{}, m_sub_expressions[0]);
            broadcast_vec(tmp.m_data.get(), new_data.get(), tmp.size(), preparatory.size);
        }

        size_t dim_i = std::visit(GetDimensions, preparatory.sub_iterators[0]).size() - 1;
        size_t width = std::visit(GetSize, preparatory.sub_iterators[0]);
        for (unsigned i=1; i < preparatory.sub_iterators.size(); ++i) {
            const size_t dim_i_previous = dim_i;
            dim_i += std::visit(GetDimensions, preparatory.sub_iterators[i]).size();

            if (std::visit(IsContiguous, preparatory.sub_iterators[i])) {
                broadcast_chunks(
                    std::visit(GetData, preparatory.sub_iterators[i]),
                    new_data.get(),
                    std::visit(GetSize, preparatory.sub_iterators[i]),
                    width
                );
            }
            else {
                const auto tmp = std::visit(GetTensor{}, m_sub_expressions[i]);
                broadcast_chunks(tmp.m_data.get(), new_data.get(), tmp.size(), width);
            }

            for (size_t j=dim_i_previous; j < dim_i; ++j) width *= preparatory.dimensions[j].size();
        }

        broadcast(m_modifier, new_data.get(), preparatory.size);

        // Set up the tensor at the end in case we're doing an assignment
        tensor.m_dimensions = preparatory.dimensions;
        tensor.m_size = preparatory.size;
        tensor.m_data.swap(new_data);
    };

    switch (Preparatory preparatory{m_sub_expressions, PRODUCT}; preparatory.state) {
    case SCALAR:
        tensor.populate_scalar(m_modifier, true);
        break;
    case FREE_MULTIPLICATION:
        populate_free_multiplication(preparatory);
        break;
    case GENERAL: {
        iterator iter{m_modifier, preparatory};
        tensor.populate_general(iter, end(), true);
        break;
    }
    default:
        throw VariTensorInternalError("Invalid expression state!");
    }
}

inline ProductOp::iterator ProductOp::begin() const {
    Preparatory preparatory{m_sub_expressions, PRODUCT};
    return iterator{m_modifier, preparatory};
}

inline ProductOp::iterator ProductOp::end() const {
    Preparatory preparatory{};
    return iterator{m_modifier, preparatory, true};
}

inline ExpressionIterator ProductOp::vbegin() const {
    return begin();
}

[[nodiscard]] inline bool ProductOp::is_scalar() const {
    return m_sub_expressions.empty();
}

[[nodiscard]] inline double ProductOp::get_scalar() const {
    return m_modifier;
}

inline void ProductOp::add_element(const double& value) {
    m_modifier *= value;
}

inline void ProductOp::add_element(const Tensor& tensor) {
    m_sub_expressions.emplace_back(View{tensor});
}

inline void ProductOp::add_element(const LinkedOp& linked_op) {
    m_sub_expressions.emplace_back(linked_op);
}

inline void ProductOp::add_element(const ProductOp& product_op) {
    m_modifier *= product_op.m_modifier;
    for (auto& expression: product_op.m_sub_expressions) m_sub_expressions.emplace_back(expression);
}

inline void ProductOp::add_element(const View& view) {
    m_sub_expressions.emplace_back(view);
}

} // namespace impl

// =================================================================================================
//                                                                                     Tensor impl |
// =================================================================================================

inline Tensor::Tensor(const std::initializer_list<VarianceQualifiedIndex> vq_indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices}
{}

inline Tensor::Tensor(const std::initializer_list<VarianceQualifiedIndex> vq_indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::initializer_list<VarianceQualifiedIndex> vq_indices):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);
}

inline Tensor::Tensor(std::string name, const std::initializer_list<VarianceQualifiedIndex> vq_indices, const double initial_value):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const std::initializer_list<Index> indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices}
{}

inline Tensor::Tensor(const std::initializer_list<Index> indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::initializer_list<Index> indices):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);

}

inline Tensor::Tensor(std::string name, const std::initializer_list<Index> indices, const double initial_value):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices}
{}

inline Tensor::Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);
}

inline Tensor::Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices, const double initial_value):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const std::vector<Index>& indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices}
{}

inline Tensor::Tensor(const std::vector<Index>& indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::vector<Index>& indices):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);
}

inline Tensor::Tensor(std::string name, const std::vector<Index>& indices, const double initial_value):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, initial_value}
{}

inline Tensor::Tensor(std::string name, const double initial_value):
    m_name{std::move(name)}
{
    m_data = impl::allocate(1);
    *m_data = initial_value;
}

inline Tensor::Tensor(const Tensor& other):
    m_dimensions{other.m_dimensions},
    m_size{other.m_size},
    m_name{other.m_name},
    m_tensor_class{other.m_tensor_class}
{
    m_data = impl::allocate(m_size);
    impl::copy(m_data.get(), other.m_data.get(), m_size);
}

inline Tensor& Tensor::operator=(const Tensor& other) {
    if (other.m_data != m_data) { // if this isn't copy-to-self
        m_name = other.m_name;
        m_dimensions = other.m_dimensions;
        m_size = other.m_size;
        m_tensor_class = other.m_tensor_class;

        impl::DoublePtr new_data = impl::allocate(m_size);
        impl::copy(new_data.get(), other.m_data.get(), m_size);
        m_data.swap(new_data);
    }

    return *this;
}

inline Tensor::Tensor(Tensor&& other) noexcept:
    m_dimensions{std::move(other.m_dimensions)},
    m_size{other.m_size},
    m_data{std::move(other.m_data)},
    m_name{std::move(other.m_name)},
    m_tensor_class{other.m_tensor_class}
{
    other.m_data = nullptr;
}

inline Tensor& Tensor::operator=(Tensor&& other) noexcept {
    if (other.m_data != m_data) { // if this isn't move-to-self
        m_name = std::move(other.m_name);
        m_dimensions = std::move(other.m_dimensions);
        m_size = other.m_size;
        m_tensor_class = other.m_tensor_class;

        m_data = std::move(other.m_data);
        other.m_data = nullptr;
    }

    return *this;
}

inline Tensor::operator double() const {
    impl::deny(m_size > 1, "Attempt to convert non-scalar tensor to double");
    return *m_data;
}

inline View Tensor::operator[](impl::Indexables indices) const {
    impl::deny(indices.size() != m_dimensions.size(), "Indexing dimension mismatch");

    size_t offset{0};
    size_t width{1};
    impl::Dimensions passed_indices;
    for (size_t i = 0; i < indices.size(); ++i) {
        if (std::holds_alternative<Index>(indices[i])) {
            impl::deny(std::get<Index>(indices[i]).size() > m_dimensions[i].index.size(),
                            "Index size mismatch");
            passed_indices.emplace_back(
                std::get<Index>(indices[i]),
                m_dimensions[i].variance,
                width
            );
            width *= std::get<Index>(indices[i]).size();
        }
        else {
            impl::deny(std::get<int>(indices[i]) >= m_dimensions[i].index.size(),
                            "Index size mismatch");
            offset += std::get<int>(indices[i]) * width;
        }
    }

    return View{*this, m_data.get() + offset, passed_indices};
}

inline double& Tensor::operator[](const std::vector<int>& indices) const {
    impl::deny(indices.size() > m_dimensions.size(), "Indexing dimension mismatch");
    auto data = m_data.get();

    int n = 0;
    for(const auto index : indices) {
        impl::deny(index < 0, "Indices cannot be less than zero");
        impl::deny(index >= m_dimensions[n].index.size(), "Index size mismatch");

        data += m_dimensions[n].width * index;
        ++n;
    }

    return *data;
}

[[nodiscard]] inline std::string Tensor::name() const {return m_name;}
[[nodiscard]] inline size_t Tensor::size() const {return m_size;}
[[nodiscard]] inline size_t Tensor::size(const int dimension) const {return m_dimensions[dimension].index.size();}
[[nodiscard]] inline int Tensor::rank() const {return static_cast<int> (m_dimensions.size());}
[[nodiscard]] inline bool Tensor::is_scalar() const {return m_size == 1;}
[[nodiscard]] inline double Tensor::get_scalar() const {return *m_data;}
[[nodiscard]] inline bool Tensor::is_metric() const {return m_tensor_class == impl::METRIC_TENSOR;}

[[nodiscard]] inline std::vector<Index> Tensor::indices() const {
    std::vector<Index> indices;
    for (const auto& dimension: m_dimensions) indices.emplace_back(dimension.index);
    return indices;
}

[[nodiscard]] inline std::vector<VarianceQualifiedIndex> Tensor::qualified_indices() const {
    std::vector<VarianceQualifiedIndex> qualified_indices;
    for (const auto& dimension: m_dimensions) qualified_indices.emplace_back(dimension.index, dimension.variance);
    return qualified_indices;
}

[[nodiscard]] inline Variance Tensor::variance(const Index& index) const {
    for (const auto& dimension: m_dimensions) {
        if (dimension.index == index){
            return dimension.variance;
        }
    }
    throw TensorLogicError(
        "Missing index when finding variance!"
    ); // can't use deny() as there is nothing to return
}

[[nodiscard]] inline Variance Tensor::variance(const int index) const {
    impl::deny(static_cast<size_t> (index) >= m_dimensions.size(), "Index out of bounds");
    return m_dimensions[index].variance;
}

[[nodiscard]] inline bool Tensor::has_index(const Index& index) const {
    return std::ranges::any_of(m_dimensions, [&](const auto& dimension) {return dimension.index == index;});
}

[[nodiscard]] inline size_t Tensor::index_position(const Index& index) const {
    for (size_t i=0; i<m_dimensions.size(); ++i) {
        if (m_dimensions[i].index == index){
            return i;
        }
    }
    throw TensorLogicError(
        "Missing index when finding index position"
    ); // can't use deny() as there is nothing to return
}

[[nodiscard]] inline const Index& Tensor::indices(const int index) const {
    impl::deny(static_cast<size_t>(index) >= m_dimensions.size(), "Index out of bounds");
    return m_dimensions[index].index;
}

inline Tensor& Tensor::set_name(const std::string& name) {
    m_name = name;
    return *this;
}

inline Tensor& Tensor::transpose(const Index& first, const Index& second) {
    impl::deny(rank() < 2, "Tensor has too few indices for transposition");
    impl::deny(first == second, "Cannot transpose identical indices");
    impl::deny(first.size() != second.size(), "Cannot transpose indices of different lengths");

    // differentiate between the transposition and static dimensions
    impl::Dimension* dim1{nullptr};
    impl::Dimension* dim2{nullptr};
    impl::Dimensions static_dims;
    for (auto & m_dimension : m_dimensions) {
        if      (m_dimension.index == first)  dim1 = &m_dimension;
        else if (m_dimension.index == second) dim2 = &m_dimension;
        else static_dims.emplace_back(m_dimension);
    }

    impl::deny(!(dim1 && dim2), "Missing index when transposing");

    const auto transpose_size = dim1->index.size(); // NOLINT - we check for nullptr in the line above
    auto transpose_slice = [&transpose_size, &dim1, &dim2](double* data) {
        for (int j = 0; j < transpose_size - 1; ++j) {
            for (int i = j + 1; i < transpose_size; ++i) {
                std::swap(
                    *(data + i*dim1->width + j*dim2->width),
                    *(data + i*dim2->width + j*dim1->width)
                );
            }
        }
    };

    if (static_dims.empty()) transpose_slice(m_data.get());
    else {
        auto static_iter = begin();
        std::vector<int> static_positions(static_dims.size());
        do transpose_slice(static_iter.data());
        while (impl::increment_positions(static_positions, static_dims, static_iter));
    }

    std::swap(dim1->index, dim2->index);
    std::swap(dim1->variance, dim2->variance);
    // the width is intentionally NOT swapped to maintain memory alignment

    return *this;
}

inline Tensor& Tensor::relabel(const Index& old_index, const Index& new_index) {
    for (auto& dimension: m_dimensions) {
        if (dimension.index == old_index) {
            impl::deny(dimension.size() != new_index.size(),
                 "Cannot relabel to an index with different size");
            dimension.index = new_index;
            return *this;
        }
    }
    throw TensorLogicError(
        "Cannot find index to relabel!"
    ); // can't use deny() as there is nothing to return
}

inline Tensor& Tensor::set_variance(const Index& index, const Variance variance) {
    for (auto& dimension: m_dimensions) {
        if (dimension.index == index) {
            dimension.variance = variance;
            return *this;
        }
    }
    throw TensorLogicError(
        "Cannot find index!"
    ); // can't use deny() as there is nothing to return
}

inline Tensor& Tensor::raise(const Index& index) {
    return set_variance(index, CONTRAVARIANT);
}

inline Tensor& Tensor::lower(const Index& index) {
    return set_variance(index, COVARIANT);
}

[[nodiscard]] inline View::iterator Tensor::begin() const {
    return View{*this}.begin();
}

[[nodiscard]] inline View::iterator Tensor::end() const {
    return View{*this}.end();
}

[[nodiscard]] inline View::const_iterator Tensor::cbegin() const {
    return View{*this}.cbegin();
}

[[nodiscard]] inline View::const_iterator Tensor::cend() const {
    return View{*this}.cend();
}

inline impl::ProductOp Tensor::operator-() const {
    return {*this, -1.0, impl::MUL};
}

inline impl::LinkedOp operator+(const Tensor& first, const Tensor& second) {
    return {first, second, impl::ADD};
}

inline impl::LinkedOp operator+(const Tensor& first, const double& second) {
    return {first, second, impl::ADD};
}

inline impl::LinkedOp operator+(const double& first, const Tensor& second) {
    return {first, second, impl::ADD};
}

inline Tensor& operator+=(Tensor& first, const Tensor& second) {
    impl::LinkedOp{first, second, impl::ADD}.populate(first, false);
    return first;
}

inline double& operator+=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot add double and non-scalar tensor");
    return first += *second.m_data;
}

inline Tensor& operator+=(Tensor& first, const double& second) {
    impl::deny(first.m_size != 1, "Cannot add non-scalar tensor and double");
    *first.m_data += second;
    return first;
}

inline impl::LinkedOp operator-(const Tensor& first, const Tensor& second) {
    return {first, second, impl::SUB};
}

inline impl::LinkedOp operator-(const Tensor& first, const double& second) {
    return {first, second, impl::SUB};
}

inline impl::LinkedOp operator-(const double& first, const Tensor& second) {
    return {first, second, impl::SUB};
}

inline Tensor& operator-=(Tensor& first, const Tensor& second) {
    impl::LinkedOp{first, second, impl::SUB}.populate(first, false);
    return first;
}

inline double& operator-=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot subtract double and non-scalar tensor");
    return first -= *second.m_data;
}

inline Tensor& operator-=(Tensor& first, const double& second) {
    impl::deny(first.m_size != 1, "Cannot subtract non-scalar tensor and double");
    *first.m_data -= second;
    return first;
}

inline impl::ProductOp operator*(const Tensor& first, const Tensor& second) {
    return {first, second, impl::MUL};
}

inline impl::ProductOp operator*(const Tensor& first, const double& second) {
    return {first, second, impl::MUL};
}

inline impl::ProductOp operator*(const double& first, const Tensor& second) {
    return {first, second, impl::MUL};
}

inline Tensor& operator*=(Tensor& first, const Tensor& second) {
    impl::ProductOp{first, second, impl::MUL}.populate(first);
    return first;
}

inline double& operator*=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot assign non-scalar tensor to double!");
    return first *= *second.m_data;
}

inline Tensor& operator*=(Tensor& first, const double& second) {
    impl::ProductOp{first, second, impl::MUL}.populate(first);
    return first;
}

inline impl::ProductOp operator/(const Tensor& first, const Tensor& second) {
    return impl::ProductOp{first, second, impl::DIV};
}

inline impl::ProductOp operator/(const Tensor& first, const double& second) {
    return {first, second, impl::DIV};
}

inline impl::ProductOp operator/(const double& first, const Tensor& second) {
    return {first, second, impl::DIV};
}

inline Tensor& operator/=(Tensor& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot divide by non-scalar tensor!");
    impl::ProductOp{first, *second.m_data, impl::DIV}.populate(first);
    return first;
}

inline double& operator/=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot divide by non-scalar tensor!");
    return first /= *second.m_data;
}

inline Tensor& operator/=(Tensor& first, const double& second) {
    impl::ProductOp{first, second, impl::DIV}.populate(first);
    return first;
}

inline bool operator==(const Tensor& first, const Tensor& second) {
    try {
        for (const auto& value: Tensor{first - second}) {
            if (value != 0) return false; // indices MUST be linked when comparing
        }
    }
    catch (std::logic_error&) {
        return false;
    }

    return true;
}

inline bool operator==(const Tensor& first, const double& second) {
    if (first.size() != 1) return false;
    return *first.m_data == second;
}

inline bool operator==(const double& first, const Tensor& second) {
    if (second.size() != 1) return false;
    return first == *second.m_data;
}

inline void Tensor::set_tensor_class(const impl::TensorClass tensor_class) {
    m_tensor_class = tensor_class;
}

inline void Tensor::populate_scalar(const double scalar, const bool allocate) {
    if (allocate) { // for LinkedOp assignment, the original tensor is already set up correctly
        m_data = impl::allocate(1);
        m_size = 1;
    }

    *m_data = scalar;
}

// =================================================================================================
//                                                                                 pretty printing |
// =================================================================================================

namespace impl {

inline size_t print_data_width = 5;
inline int print_data_precision = 1;

// every entry in this string must be EXACTLY 5 lines; the name and width of each
// letter must be recorded in the subsequent letter_widths map
const auto letters = R"(
  __ ,
/ _  |
\__, |
\___/

 ___
/ __)
> _)
\___)
 / ,__|
 \ \
 ,  \
| () |
 \__/
,_______
\  ,---.|
 | |   L|
 | |
 |_|





,-,
|/







 ____



 ----

   _
 _| |_
|_   _|
  |_|

\|/
/|\



_.
\ \
 \ \
  \ \
   \_\
    ,_,
   / /
  / /
 / /
/_/

 _____
|_____|
 _____
|_____|
(_) ,_,
   / /
  / /
 / / _
/_/ ( )

    _
  /  \
 / /\ \
/_/  \_\



 _
'_'



,_,
|/
  /
 |
 |
 |
  \
\
 |
 |
 |
/
 ,-
 |
 |
 |
 '_
-.
 |
 |
 |
_'
 ,-
 |
<
 |
 '_
-.
 |
  >
 |
_'
    _
   / \
  / _ \
 / ___ \
/_/   \_\
 ____
|  _ )
|  _ \
| |_) |
|____/
  ____
 / ___|
| |
| |___
 \____|
 ____
|  _ \
| | | |
| |_| |
|____/
 _____
| ____|
|  _|
| |___
|_____|
 _____
|  ___|
| |_
|  _|
|_|
  ____
 / ___|
| |  _
| |_| |
 \____|
 _   _
| |_| |
|  _  |
| | | |
|_| |_|
 ___
|_ _|
 | |
 | |
|___|
   ___
  |__ |
 _  | |
| |_| |
 \___/
 _  __
| |/ /
| ' /
| . \
|_|\_\
 _
| |
| |
| |___
|_____|
 __  __
|  \/  |
| |\/| |
| |  | |
|_|  |_|
 _   _
| \ | |
|  \| |
| |\  |
|_| \_|
  ___
 / _ \
| | | |
| |_| |
 \___/
 ____
|  _ \
| |_) |
|  __/
|_|
  ___
 / _ \
| | | |
| |_| |
 \__\_\
 ____
|  _ \
| |_) |
|  _ <
|_| \_\
 ____
/ ___|
\___ \
 ___) |
|____/
 _____
|_   _|
  | |
  | |
  |_|
 _   _
| | | |
| | | |
| |_| |
 \___/
 _     _
| |   | |
 \ \ / /
  \ V /
   \_/
 _      _
| |    | |
| \ /\ / |
 \ V  V /
  \_/\_/
__  __
\ \/ /
 \  /
 /  \
/_/\_\
__   __
\ \ / /
 \ V /
  | |
  |_|
_____
|__  /
  / /
 / /_
/____|


 __,
/  |
\__|,
 |
 |
 |__
 |  \
,|__/


 ,_
/
\__,
   |
   |
 __|
/  |
\__|,


 __
/__|
\__
  _
 / `
-|-
 |
 |


,__,
\__|
___/
|
|
| _
|/ |
|  |

 .
 _
 |
 \_
  .
  _
  |
  |
\_/
|
|__
|  \
|__/
|  \
 |
 |
 |
 |
 \___


, _  _
|/ |/ |
|  |  |


, _
|/ |
|  |


 __
/  \
\__/


,__,
|__/
|


,__,
\__|
   |/


. ,-
 |
 |


 __
|__,
.__|

_|_
 |
 |
 \_


_   _
|   |
|__/|,


.    ,
 \  /
  \/


.        ,
 \  /\  /
  \/  \/


\ /
 x
/ \


,   _
\__/|
 ___/


___
 \
__\
)";

// controls the width of each letter when printed; this is necessary to
// make every line of each letter the same width to prevent misalignment
const std::vector<std::pair<int, int>> letter_widths = {
    {METRIC_TENSOR, 6},
    {LEVI_CIVITA_SYMBOL, 5},
    {KRONECKER_DELTA, 7},
    {CHRISTOFFEL_SYMBOL, 9},
    {' ', 3},
    {'\'', 3},
    {'_', 6},
    {'-', 6},
    {'+', 7},
    {'*', 3},
    {'\\', 6},
    {'/', 6},
    {'=', 7},
    {'%', 7},
    {'^', 8},
    {'.', 3},
    {',', 3},
    {'(', 3},
    {')', 3},
    {'[', 3},
    {']', 3},
    {'{', 3},
    {'}', 3},
    {'A', 9},
    {'B', 7},
    {'C', 7},
    {'D', 7},
    {'E', 7},
    {'F', 7},
    {'G', 7},
    {'H', 7},
    {'I', 5},
    {'J', 7},
    {'K', 6},
    {'L', 7},
    {'M', 8},
    {'N', 7},
    {'O', 7},
    {'P', 7},
    {'Q', 7},
    {'R', 7},
    {'S', 7},
    {'T', 7},
    {'U', 7},
    {'V', 9},
    {'W', 10},
    {'X', 6},
    {'Y', 7},
    {'Z', 6},
    {'a', 5},
    {'b', 5},
    {'c', 4},
    {'d', 5},
    {'e', 4},
    {'f', 4},
    {'g', 4},
    {'h', 4},
    {'i', 4},
    {'j', 4},
    {'k', 4},
    {'l', 5},
    {'m', 7},
    {'n', 4},
    {'o', 4},
    {'p', 4},
    {'q', 5},
    {'r', 4},
    {'s', 4},
    {'t', 4},
    {'u', 7},
    {'v', 6},
    {'w', 10},
    {'x', 3},
    {'y', 5},
    {'z', 4}
};

inline void add_letter(std::vector<std::string>& lines, const int letter) {
    auto* letter_ptr = letters;

    for (auto [cur_letter, width]: letter_widths) {
        if (cur_letter == letter) {
            // we found the letter we need, so output it and return
            for (auto& line: lines) {
                for (int i = 0; i < width; ++i) {
                    if (*letter_ptr != '\n') {
                        line += *letter_ptr;
                        ++letter_ptr;
                    }
                    else line += ' ';
                }
                ++letter_ptr;
            }
            return;
        }

        // step 5 lines to get to the next letter
        int step = 5;
        while (step) {
            if (*letter_ptr == '\n') --step;
            ++letter_ptr;
        }
    }
    // if we didn't find the letter, just skip
}

inline void add_header_name(
    const std::string& name,
    std::vector<std::string>& head_lines,
    const TensorClass tensor_class
) {
    if (tensor_class != TENSOR) { // some tensors have special names, e.g. the kronecker delta
        add_letter(head_lines, tensor_class);
    }
    else {
        for (const auto letter: name) {
            add_letter(head_lines, letter);
        }
    }
}

inline void add_header_indices(const Dimensions& dimensions, std::vector<std::string>& header) {
    for (auto& dimension: dimensions) {
        std::string blank;
        for(size_t i=0; i<dimension.index.name().size(); ++i) blank += " ";

        if (dimension.variance == CONTRAVARIANT){ // Upper index
            header[1] += dimension.index.name() + " ";
            header[3] += blank + " ";
            header[4] += blank + " ";
        }
        else { // Lower index
            header[1] += blank + " ";
            header[3] += blank + " ";
            header[4] += dimension.index.name() + " ";
        }
    }
}

inline void write_header(
    std::ostream& stream,
    const Dimensions& dimensions,
    const std::string& name,
    const TensorClass tensor_type
) {
    /** Outputs the "title" of the tensor in fancy lettering with the indices in their correct positions
     * e.g.   ______
     *       |__  __|
     *         | |    mu
     *         | |          =
     *         |_|      nu
     */

    auto add_space = [](std::vector<std::string>& header) {
        for(auto& line: header) line += " ";
    };

    auto add_equals_sign = [&](std::vector<std::string>& header) {
        if (tensor_type == METRIC_TENSOR) header[3] += "  ";
        else header[3] += " =";
    };

    auto output_header = [&](const std::vector<std::string>& header) {
        for (int i = 0; i < 5; ++i) stream << header[i] << "\n";
    };

    std::vector<std::string> header{5};

    add_header_name(name, header, tensor_type);
    add_space(header);
    add_header_indices(dimensions, header);
    add_equals_sign(header);
    output_header(header);
}

inline void write_subtitle(std::ostream& stream, const Tensor& tensor, const TensorClass print_type) {
    /** writes a small note of what "sort" of tensor we have
     * e.g. "Rank 5 Tensor", "Column Vector", etc.
     */
    if (print_type == METRIC_TENSOR) stream << "Metric Tensor\n\n";
    else if (print_type == LEVI_CIVITA_SYMBOL) stream << "Levi-Civita Symbol\n\n";
    else if (print_type == KRONECKER_DELTA) stream << "Kronecker Delta\n\n";
    else {
        if (const auto rank = tensor.rank(); rank == 0) stream << "Scalar Tensor\n\n";
        else if (rank == 1) {
            if (tensor.variance(0) == CONTRAVARIANT) stream << "Column";
            else stream << "Row";
            stream << " Vector\n\n";
        }
        else if (rank == 2) stream << " Matrix Tensor\n\n";
        else stream << "Rank " << rank << " Tensor\n\n";
    }
}

inline std::string format_value(const double value) {
    std::stringstream stream;
    stream << std::fixed << std::setprecision(print_data_precision) << value;
    auto result = stream.str();

    // pad any strings shorter than print_data_width
    if (result.size() < print_data_width) {
        result.insert(0, std::string(print_data_width - result.size(), ' '));
    }

    // truncate any strings longer than print_data_width and add the truncation character (~)
    else if (result.size() > print_data_width) {
        result = result.substr(0, print_data_width -1) + "~"; // -1 to accommodate ~
    }

    return result;
}

inline void output_row(
    std::ostream& stream,
    const double* data,
    const size_t size
) {
    // for scalar and row vector
    stream << "< ";
    for (size_t i=0; i<size; ++i) stream << format_value(data[i]) << " ";
    stream << ">" << "\n";
}

inline void output_column(
    std::ostream& stream,
    const double* data,
    const size_t size
) {
    // for scalar and row vector
    stream << " /" << format_value(data[0]) << "\\\n" ;
    for (size_t i=1; i<size-1; ++i) stream << "| " << format_value(data[i]) << " |\n";
    stream << " \\" << format_value(data[size-1]) << "/\n" ;
}

inline void output_grid(
    const std::vector<std::vector<std::string>>& value_matrix,
    std::ostream& stream,
    const int total_width,
    const int total_height,
    const int i_size,
    const int k_size
) {
    std::vector<std::stringstream> output_rows(total_height + 2); // +2 for the bracket heads at the top and bottom
    const int n_whitespace = i_size * k_size + k_size; // spaces between characters + spaces between slices
    const int total_fields_length = total_width * static_cast<int>(print_data_width);
    const int total_matrix_width = total_fields_length + n_whitespace - 1;

    // add the starting bracket
    output_rows[0] << " /";
    for (int ii=1; ii<total_height+1; ++ii) output_rows[ii] << "|";
    output_rows[total_height+1] << " \\";

    // add the value data in between
    output_rows[0] << std::string(total_matrix_width, ' ');
    for (size_t ii=1; ii<output_rows.size()-1; ++ii) {
        for (int jj=0; jj<total_width; ++jj) {
            if (jj % i_size == 0) output_rows[ii] << " "; // adds an extra space before every slice
            output_rows[ii] << value_matrix[ii - 1][jj] << " "; // note the space between every field
        }
    }
    output_rows[total_height+1] << std::string(total_matrix_width, ' ');

    // add the ending bracket
    output_rows[0] << "\\\n";
    for (int ii=1; ii<total_height+1; ++ii) output_rows[ii] << " |\n";
    output_rows[total_height+1] << "/\n";

    // finally, output all the rows
    for (auto& row: output_rows) stream << row.str();
}

inline void output_234(const Tensor& tensor, const Dimensions& dimensions, std::ostream& stream) {
    // for tensors of rank 2, 3, or 4 (by far the most complicated case)

    const int i_size = dimensions[0].index.size();
    const int j_size = dimensions[1].index.size();
    const int k_size = dimensions.size() >= 3 ? dimensions[2].index.size() : 1;

    const int n_major_row = dimensions.size() == 4 ? dimensions[3].size() : 1; // number of big grids to output
    const int n_squares = dimensions.size() >= 3 ? dimensions[2].size() : 1; // number of squares in each grid

    // create a set of indices to slice each square out of the tensor
    Indexables indices;
    switch (dimensions.size()) {
        default:
            indices.insert(indices.begin(), -1);
            [[fallthrough]];
        case 3:
            indices.insert(indices.begin(), -1);
            [[fallthrough]];
        case 2:
            indices.insert(indices.begin(), dimensions[1].index);
            indices.insert(indices.begin(), dimensions[0].index);
            break;
    }

    // make a string matrix to store the values in
    // storing in height-by-width form makes the output stage easier
    const int total_width = i_size * n_squares;
    const int total_height = j_size + n_squares - 1;
    std::vector value_matrix(
        total_height,
        std::vector(total_width, std::string(print_data_width, ' '))
    );

    for (int i=0; i<n_major_row; ++i) {
        // for each major row
        if (dimensions.size() == 4) indices[3] = i;

        int start_height = total_height - j_size;
        int start_width = 0;
        for (int j=0; j<n_squares; ++j) { // for each square in the major row
            if (dimensions.size() >= 3) indices[2] = j;

            // slice the tensor and fill in the slice's values starting at (start_height, start_width)
            Tensor square = tensor[indices];
            for (int jj=0; jj<j_size; ++jj) {
                for (int ii=0; ii<i_size; ++ii) {
                    value_matrix[start_height+jj][start_width + ii] = format_value(square[ii, jj]);
                }
            }

            // move the starting position to the next square
            start_width += i_size;
            --start_height;
        }

        output_grid(value_matrix, stream, total_width, total_height, i_size, k_size);
    }
}

} // namespace impl

/**
 * @brief Set the width of the fields when geometric pretty-printing
 *
 * Any values on the tensor longer than this width will be truncated with a ~ symbol.
 */
inline void set_print_data_width(const int width) {
    // sets the width of the data elements when pretty printing
    impl::print_data_width = width;
}

/**
 * @brief Set the number of significant figures to round values to when pretty-printing
 */
inline void set_print_precision(const int precision) {
    // set the precision of the output stream when pretty printing
    impl::print_data_precision = precision;
}

inline void write_data(std::ostream& stream, const Tensor& tensor) {
    // outputs just the tensor data in pretty format
    if(tensor.m_dimensions.empty()) impl::output_row(stream, tensor.m_data.get(), tensor.m_size);
    else if(tensor.m_dimensions.size() == 1) {
        if (tensor.variance(0) == CONTRAVARIANT) impl::output_column(stream, tensor.m_data.get(), tensor.m_size);
        else impl::output_row(stream, tensor.m_data.get(), tensor.m_size);
    }
    else if (tensor.m_dimensions.size() <= 4) impl::output_234(tensor, tensor.m_dimensions, stream);
    else tensor.dump(stream);
}

inline std::ostream& pretty_print(std::ostream& ostream, const Tensor& tensor) {
    // display the tensor in detailed geometric format, including name and index information
    impl::write_header(ostream, tensor.m_dimensions, tensor.name(), tensor.m_tensor_class);
    impl::write_subtitle(ostream, tensor, tensor.m_tensor_class);
    write_data(ostream, tensor);
    return ostream;
}

}  // namespace varitensor

inline std::ostream& operator<<(std::ostream& stream, const varitensor::Tensor& tensor) {
    return varitensor::pretty_print(stream, tensor);
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::LinkedOp operator+(const T& first, const U& second) {
    return {first, second, varitensor::impl::ADD};
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::LinkedOp operator-(const T& first, const U& second) {
    return {first, second, varitensor::impl::SUB};
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::ProductOp operator*(const T& first, const U& second) {
    return {first, second, varitensor::impl::MUL};
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::ProductOp operator/(const T& first, const U& second) {
    return {first, second, varitensor::impl::DIV};
}

#endif // VARITENSOR_H
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * Licence, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#ifndef VARITENSOR_H
#define VARITENSOR_H

#include <algorithm>
#include <cmath>
#include <cstring>
#include <functional>
#include <iomanip>
#include <map>
#include <memory>
#include <numeric>
#include <ranges>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <variant>
#include <vector>

#ifndef VARITENSOR_VALIDATION_ON
#define VARITENSOR_VALIDATION_ON 1
#endif

namespace varitensor {

class Tensor;
class View;
class Index;

/**
 * @brief Represents a logic error in the requested tensor operations
 *
 * This normally means a caller requested an operation that doesn't make sense mathematically, like trying to transpose
 * two indices of different sizes.
 */
struct TensorLogicError final: std::logic_error {
    explicit TensorLogicError(const std::string& message):
        std::logic_error("<Vari-Tensor> Tensor logic error: " + message)
    {}
};

namespace impl {

template <bool is_const>
class ViewIterator;

class LinkedOp;
class LinkedOpIterator;
class ProductOp;
class ProductOpIterator;
struct Preparatory;

constexpr int MAX_INTERVAL = -1;

// NB: we occasionally use the fact that ADD == -SUB
enum Operation {SUB=-1, ADD=1, MUL=2, DIV=3};

enum ExprState {
    SCALAR,
    ALIGNED_INDICES,
    FREE_MULTIPLICATION,
    GENERAL
};

enum PreparatoryType {
    LINKED,
    PRODUCT
};

inline void deny(const bool condition, const std::string& message) {
    if constexpr (VARITENSOR_VALIDATION_ON) {
        if (condition) [[unlikely]] {
            throw TensorLogicError(message);
        }
    }
}

inline auto VBegin = [](auto& expression) {return expression.vbegin();};
inline auto GetDimensions = [](auto& expression) {return expression.dimensions();};
inline auto GetSize = [](auto& expression) {return expression.size();};
inline auto GetData = [](auto& iter) {return iter.data();};

using DoublePtr = std::unique_ptr<double, void(*)(double*)>;

} // namespace impl

/**
 * @brief Helper alias to provide the correct signature for user-defined metric functions
 */
using MetricFunction = std::function<double(int, int)>;

/**
 * @brief Convenience aliases for setting index size
 */
enum IndexSizes {LATIN = 3, GREEK = 4};

/**
 * @brief Aliases used to express index variance
 */
enum Variance {COVARIANT = 0, LOWER = 0, CONTRAVARIANT = 1, UPPER = 1};

/**
 * @brief Expresses an interval on an index
 *
 * Both first and last are included in the range of the interval.
 */
struct Interval {
    const Index& origin;
    const int first;
    const int last;
};

/**
 * @brief Represents an index of a tensor
 */
class Index {
public:
/**
 * @brief Initialise an index
 *
 * @param size Index size; must be greater than 1.
 */
///@{
    explicit Index(const int size):
        m_size{size},
        m_id{s_next_id++}
    {
        impl::deny(m_size < 2, "Cannot initialize unnamed index with size < 2");
    }

    explicit Index(std::string  name, const int size):
        m_size{size},
        m_id{s_next_id++},
        m_name{std::move(name)}
    {
        impl::deny(m_size < 2, "Cannot initialize index with size < 2");
    }
///@}

/**
 * @brief Create a new index from an interval
 */
    Index(const Interval& interval): // NOLINT - we want implicit conversion
        m_size{interval.last - interval.first + 1},
        m_id{interval.origin.m_id},
        m_name{interval.origin.m_name},
        m_interval_start{interval.first}
    {}

/**
 * @brief Compare two indices
 *
 * Note that for indices the name is included in the comparison.
 */
    bool operator==(const Index& other) const {
        if (m_name.empty()) return m_id == other.m_id && m_size == other.m_size;
        return m_name == other.m_name && m_size == other.m_size;
    }

    [[nodiscard]] const int& size() const { return m_size; }

    [[nodiscard]] const int& id() const { return m_id; }

    [[nodiscard]] std::string name() const {
        std::ostringstream name;

        if (m_name.empty()) name << "idx" << m_id;
        else name << m_name;

        if (m_interval_start != -1) {
            name << std::string("(") << m_interval_start << '-' << (m_interval_start + m_size - 1) << ')';
        }

        return name.str();
    }

    void set_name(const std::string& name) { m_name = name; }

/**
 * @brief Use when slicing a tensor to select an interval of an index
 *
 * e.g. T[ index(1) ] will get a slice ignoring the 0th elements of T.
 *
 * @param first The first value to include; must be greater than 0.
 * @param last The last index to include; takes the maximum index value by default.
 */
    ///@{
    [[nodiscard]] Interval operator()(const int first, const int last=impl::MAX_INTERVAL) const {
        return interval(first, last);
    }

    [[nodiscard]] Interval interval(const int first, int last=impl::MAX_INTERVAL) const {
        if (last == impl::MAX_INTERVAL) last = m_size - 1;

        impl::deny(first < 0, "Interval cannot have negative start");
        impl::deny(first >= last, "Interval must end after it starts");
        impl::deny(last >= m_size, "Interval cannot overflow index size");
        return {*this, first, last};
    }
///@}

private:
    int m_size;
    int m_id;
    std::string m_name{};
    int m_interval_start{-1};

    inline static int s_next_id{0};
};

/**
 * @brief Represents an index with a variance
 *
 * Used when constructing tensors.
 */
struct VarianceQualifiedIndex {
    Index index;
    Variance variance = COVARIANT;

    bool operator==(const VarianceQualifiedIndex& other) const {
        return index == other.index && variance == other.variance;
    }
};

namespace impl {
struct Dimension {
    Index index;
    Variance variance{};
    size_t width{}; // the memory width of a single increment along this dimension

    [[nodiscard]] int size() const {return index.size();}
};

using Dimensions = std::vector<Dimension>;

template<typename T>
concept Expression_c = std::same_as<T, View> || std::same_as<T, LinkedOp> || std::same_as<T, ProductOp>;
using Expression = std::variant<View, LinkedOp, ProductOp>;
using Expressions = std::vector<Expression>;

template<typename T>
concept ExpressionIterator_c =
    std::same_as<T, ViewIterator<true>>  ||
    std::same_as<T, ViewIterator<false>> ||
    std::same_as<T, LinkedOpIterator>    ||
    std::same_as<T, ProductOpIterator>;
using ExpressionIterator = std::variant<ViewIterator<true>, ViewIterator<false>, LinkedOpIterator, ProductOpIterator>;
using ExpressionIterators = std::vector<ExpressionIterator>;

template <typename T>
concept ExpressionOperand_c =
    std::is_same_v<T, Tensor>   ||
    std::is_same_v<T, View>     ||
    std::is_same_v<T, LinkedOp> ||
    std::is_same_v<T, ProductOp> ||
    std::is_same_v<T, double>;

struct Reset {
    explicit Reset(const int index_id_): index_id{index_id_} {}

    template <ExpressionIterator_c E>
    void operator()(const E& iter) const {iter.reset(index_id);}

    int index_id;
};

struct Increment {
    explicit Increment(const int index_id_): index_id{index_id_} {}

    template <ExpressionIterator_c E>
    void operator()(const E& iter) const {iter.increment(index_id);}

    int index_id;
};

template<typename... Types>
concept AllInt_c = (... && std::is_same_v<Types, int>);

template<typename T>
concept Indexable_c = std::is_integral_v<T> || std::is_same_v<T, Index> || std::is_same_v<T, Interval>;

template<typename T>
concept Indices_c =
    std::is_same_v<std::decay_t<T>, std::vector<Index>> ||
    std::is_same_v<std::decay_t<T>, std::initializer_list<Index>> ||
    std::is_same_v<std::decay_t<T>, std::vector<VarianceQualifiedIndex>> ||
    std::is_same_v<std::decay_t<T>, std::initializer_list<VarianceQualifiedIndex>>;

template<typename T>
concept VarianceQualifiedIndices_c =
    std::is_same_v<std::decay_t<T>, std::vector<VarianceQualifiedIndex>> ||
    std::is_same_v<std::decay_t<T>, std::initializer_list<VarianceQualifiedIndex>>;

const std::string TENSOR_DEFAULT_NAME = "Vari-Tensor";

enum TensorClass {
    TENSOR,
    METRIC_TENSOR,
    KRONECKER_DELTA,
    LEVI_CIVITA_SYMBOL,
    CHRISTOFFEL_SYMBOL
};

/**
 * @brief Type aliases to help with indexing tensors
 */
///@{
using Indexable = std::variant<int, Index>;
using Indexables = std::vector<Indexable>;
///@}

void deallocate(double* data);
DoublePtr allocate(size_t size);

struct Preparatory {
    ExprState state{SCALAR};
    Dimensions dimensions;
    Dimensions repeated; // minor pollution from ProductOpIterator, but unlikely to affect performance
    ExpressionIterators sub_iterators;
    size_t size{1};

    Preparatory() = default;
    Preparatory(const Expressions& sub_expressions, PreparatoryType type);

    struct Couple {
        int dims[2]{-1, -1};
        int metric_index{-1};
        int other_index{-1};

        void add(const int dimension, const bool is_metric) {
            deny(dims[0] != -1 && dims[1] != -1,
                   "Indices in multiplication expression cannot appear more than twice");
            const auto target = dims[0] == -1 ? 0 : 1;
            dims[target] = dimension;
            if (!is_metric) other_index = dimension;
        }

        void clear() {
            dims[0] = -1;
            dims[1] = -1;
            metric_index = -1;
        }

        [[nodiscard]] bool has() const {
            return dims[0] != -1;
        }

        [[nodiscard]] bool is_repeated() const {
            return dims[1] != -1;
        }

        [[nodiscard]] bool is_metric() const {
            return metric_index != -1;
        }
    };

private:
    void prepare_linked_operation(const Expressions& sub_expressions);
    void prepare_product_operation(const Expressions& sub_expressions);
};

class ExpressionIteratorBase {
public:
    using iterator_category = std::forward_iterator_tag;
    using difference_type   = std::ptrdiff_t;
    using value_type        = double;

    ExpressionIteratorBase() = default;

    explicit ExpressionIteratorBase(Preparatory& preparatory):
        m_size{preparatory.size},
        m_dimensions{std::move(preparatory.dimensions)},
        m_positions(m_dimensions.size(), 0)
    {}

    [[nodiscard]] std::vector<int>& positions() {return m_positions;}

    [[nodiscard]] int positions(const int index) const {return m_positions[index];}

    [[nodiscard]] int positions(const Index& index) const {
        for (size_t i = 0; i < m_dimensions.size(); ++i) {
            if (m_dimensions[i].index == index) return m_positions[i];
        }
        throw TensorLogicError(
            "Unable to find index!"
        );  // can't use deny() as there is nothing to return
    }

    [[nodiscard]] const Dimensions& dimensions() const {return m_dimensions;}
    [[nodiscard]] size_t size() const {return m_size;}
    [[nodiscard]] bool is_scalar() const {return m_size == 1;}

protected:
    friend class varitensor::Tensor;

    size_t m_size{1};
    Dimensions m_dimensions;
    std::vector<int> m_positions;
};

template<bool is_const=false>
class ViewIterator: public ExpressionIteratorBase {
public:
    ViewIterator() = default;
    ViewIterator(const Tensor* target, double* data_ptr, const Dimensions& dimensions):
        m_target{target},
        m_data_ptr{data_ptr}
    {
        std::unordered_map<int, int> counts;
        for (auto& dimension: dimensions) counts[dimension.index.id()] += 1;

        for (auto& dimension: dimensions) {
            deny(counts[dimension.index.id()] > 2,
                      "Indices in contraction expressions cannot appear more than twice");

            if (counts[dimension.index.id()] == 2) { // repeated index
                m_repeated.push_back(dimension);
                m_repeated_positions.push_back(0);
                counts[dimension.index.id()] = -1;
            }
            else if (counts[dimension.index.id()] == 1) { // non-repeated index
                m_dimensions.push_back(dimension);
                m_size *= dimension.size();
                m_positions.push_back(0);
            }

            m_widths[dimension.index.id()] += {
                dimension.width,
                dimension.width * (dimension.size() -1)
            };
        }

        if (!m_dimensions.empty()) {
            m_cached_id = m_dimensions.front().index.id();
            m_cached_info = m_widths[m_cached_id];
        }
    }

    ViewIterator(const ViewIterator& other) = default;
    ViewIterator& operator=(const ViewIterator& other) = default;
    ViewIterator(ViewIterator&& other) noexcept = default;
    ViewIterator& operator=(ViewIterator&& other) = default;

    ViewIterator& operator++() {
        if (!increment_positions(m_positions, m_dimensions, *this)) {
            m_data_ptr = nullptr;
        }
        return *this;
    }

    ViewIterator operator++(int) {
        ViewIterator copy = *this;
        ++*this;
        return copy;
    }

    double& operator*() const requires(!is_const) {
        deny(is_contracted(),
             "Cannot dereference non-const contracted iterator - use std::as_const or cbegin/cend for const iteration");
        return *m_data_ptr;
    }

    double operator*() const requires(is_const) {
        return deref();
    }


    template<typename T>
    requires std::is_same_v<T, ViewIterator<>> || std::is_same_v<T, ViewIterator<true>>
    bool operator==(const T& other) const {
        return m_data_ptr == other.m_data_ptr;
    }

    [[nodiscard]] bool is_contracted() const { // used to enforce the fact that contractions can only be r-values
        return !m_repeated.empty();
    }

    bool is_metric() const;

    [[nodiscard]] bool finished() const {
        return m_data_ptr == nullptr;
    }

    void increment(const int index_id) { // NOLINT - "increment()" const is only pseudo-const
        std::as_const(*this).increment(index_id); // std::as_const needed to avoid recursion
    }

    void reset(const int index_id) { // NOLINT - "reset() const" is only pseudo-const
        std::as_const(*this).reset(index_id); // std::as_const needed to avoid recursion
    }

    [[nodiscard]] double deref() const {
        /* Despite modifying m_repeated_positions, this function is const as it always returns the positions to 0 */
        double sum = 0;

        do sum += *m_data_ptr;
        while (increment_positions(m_repeated_positions, m_repeated, *this));

        return sum;
    }

    bool is_contiguous() const {
        if (is_contracted()) return false; // contracted views counted as non-contiguous

        size_t expected_width = 1;
        for (const auto& dimension: m_dimensions) {
            if (dimension.width != expected_width) return false;
            expected_width *= dimension.size();
        }
        return true;
    }

    [[nodiscard]] double* data() const {
        return m_data_ptr;
    }

private:
    struct WidthInfo {
        size_t width{0};
        size_t total{0};

        WidthInfo& operator+=(WidthInfo&& other) {
            width += other.width;
            total += other.total;
            return *this;
        }
    };

    void increment(const int index_id) const { // technically non-const but has to be callable from deref()
        if (index_id == m_cached_id) [[likely]] m_data_ptr += m_cached_info.width;
        else if (m_widths.contains(index_id)) m_data_ptr += m_widths.at(index_id).width;
    }

    void reset(const int index_id) const { // technically non-const but has to be callable from deref()
        if (index_id == m_cached_id) [[likely]] m_data_ptr -= m_cached_info.total;
        else if (m_widths.contains(index_id)) m_data_ptr -= m_widths.at(index_id).total;
    }

    friend class ViewIterator<!is_const>;
    friend struct Reset;
    friend struct Increment;

    template <ExpressionIterator_c E>
    friend bool increment_positions(std::vector<int>& positions, const std::vector<Dimension>& dimensions, const E& iterator);

    const Tensor* m_target{nullptr};
    mutable double* m_data_ptr{nullptr}; // mutable for deref

    std::map<int, WidthInfo> m_widths;

    // profiling has indicated that map lookup can be a bottleneck: these caches are used to reduce this
    int m_cached_id{-1};
    WidthInfo m_cached_info;

    // for index contraction
    Dimensions m_repeated;
    mutable std::vector<int> m_repeated_positions; // mutable for deref
};

} // namespace impl

/**
 * @brief Represents a view on a tensor using the provided indices
 */
class View {
public:
/**
 * @brief Initialise a plain view on a tensor
 */
    explicit View(const Tensor& target); // a plain view on a tensor
    View(const Tensor& target, double* data_ptr, impl::Dimensions dimensions); // an offset view

/**
 * @brief Assign a slice of a tensor to take the values from another tensor
 */
    void operator=(const Tensor& other) &&; // NOLINT - allows T[i, 2] = U

    template<typename T = void>
    void operator=(const Tensor&) const & { // NOLINT - this definition only exists to document incorrect syntax
        static_assert(!std::is_same_v<T, T>,
            "View assignment is only valid on rvalue Views (e.g. tensor[i, j] = other). "
            "Did you mean to assign to the underlying Tensor instead?");
    }

    bool operator==(const View& other) const;

    using iterator = impl::ViewIterator<>;
    using const_iterator = impl::ViewIterator<true>;

/**
 * @brief Standard iterator and const iterator functions
 */
///@{
    [[nodiscard]] iterator begin();
    [[nodiscard]] iterator end();
    [[nodiscard]] const_iterator begin() const;
    [[nodiscard]] const_iterator end() const;
    [[nodiscard]] const_iterator cbegin() const;
    [[nodiscard]] const_iterator cend() const;
///@}

    [[nodiscard]] impl::ExpressionIterator vbegin() const;

    [[nodiscard]] double* data() const;
    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;

    void populate(Tensor& tensor, bool allocate = true) const;

private:
    const Tensor& m_target;
    double* m_data_ptr; // allows us to store an offset
    impl::Dimensions m_dimensions;
};

/**
 * @brief Represents a generalised mathematical tensor
 *
 * This class is capable of representing tensors of any rank, including scalars. It can also represent metric tensors
 * when created via varitensor::metric_tensor().
 *
 * The dimensions of a tensor are always aware of their variance, though the user can ignore this, in which case the
 * variance defaults to covariant.
 */
class Tensor {
public:
/**
 * @brief Construct a tensor with a collection of indices
 *
 * Each constructor version can be called with a name, index information, and/or initial fill value (0 by default),
 * with the only required field being the index information.
 *
 * The index information must be one of the following:
 *  - A std::initializer_list of varitensor::VarianceQualifiedIndex
 *        e.g. Tensor{ {index1, COVARIANT},  {index2, CONTRAVARIANT} }
 *  - A std::initializer_list of varitensor::Index
 *        e.g. Tensor{ index1, index2 }
 *  - A vector of varitensor::VarianceQualifiedIndex
 *        e.g. Tensor{ variance_qualified_index_vector }
 *  - A vector of varitensor::Index
 *        e.g. Tensor{ index_vector }
 *
 * The following example exercises the full functionality to create a two-dimensional tensor of ones called "T":
 *
 * @code{.cpp}
 * auto tensor = Tensor{
 *     "T",
 *     {
 *         {index1, COVARIANT},
 *         {index2, CONTRAVARIANT}
 *     },
 *     1
 * };
 * @endcode
 */
///@{
    // std::initializer_list<VarianceQualifiedIndex>
    Tensor(std::initializer_list<VarianceQualifiedIndex> vq_indices);
    Tensor(std::initializer_list<VarianceQualifiedIndex> vq_indices, double initial_value);
    Tensor(std::string name, std::initializer_list<VarianceQualifiedIndex> vq_indices);
    Tensor(std::string name, std::initializer_list<VarianceQualifiedIndex> vq_indices, double initial_value);

    // std::initializer_list<Index>
    Tensor(std::initializer_list<Index> indices);
    Tensor(std::initializer_list<Index> indices, double initial_value);
    Tensor(std::string name, std::initializer_list<Index> indices);
    Tensor(std::string name, std::initializer_list<Index> indices, double initial_value);

    // std::vector<VarianceQualifiedIndex>&
    explicit Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices);
    Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices, double initial_value);
    Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices);
    Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices, double initial_value);

    // std::vector<Index>&
    explicit Tensor(const std::vector<Index>& indices);
    Tensor(const std::vector<Index>& indices, double initial_value);
    Tensor(std::string name, const std::vector<Index>& indices);
    Tensor(std::string name, const std::vector<Index>& indices, double initial_value);
///@}

/**
 * @brief Initialise a scalar
 */
///@{
    explicit Tensor(double initial_value);
    Tensor(std::string name, double initial_value);
///@}

    template<typename E>
    requires impl::Expression_c<std::remove_cvref_t<E>> // "requires impl::Expression_c" to keep the forwarding reference
    Tensor(E&& expression) { // NOLINT - we want implicit conversion
        expression.populate(*this);

        if constexpr (!std::is_same_v<std::decay_t<E>, impl::LinkedOp>) {
            // for a linked operation, the widths are already correct
            size_t width = 1;
            for (auto& dimension: m_dimensions) {
                dimension.width = width;
                width *= dimension.size();
            }
        }
    }

    ~Tensor() noexcept = default;

    Tensor(const Tensor& other);
    Tensor& operator=(const Tensor& other);

    Tensor(Tensor&& other) noexcept;
    Tensor& operator=(Tensor&& other) noexcept;

/**
 * @brief Convert a scalar tensor to a double; throws if the tensor is not scalar.
 */
    explicit operator double() const;

/**
 * @brief Get a view on a slice of a tensor
 *
 * This function can generically slice a tensor with any combination of integers, Indices, and Intervals. Repeated
 * indices are summed over, according to the Einstein summation convention.
 *
 * @return An iterable, Tensor-convertible View object representing the slice.
 */
    template<impl::Indexable_c... Indices>
    requires (!impl::AllInt_c<Indices...>)
    [[nodiscard]] View operator[](Indices... indices) const {
        impl::deny(sizeof...(indices) != m_dimensions.size(),
                        "Indexing dimension mismatch");

        size_t n{0};
        size_t offset{0};
        impl::Dimensions passed_indices;
        construct_passed_indices(n, offset, passed_indices, indices...);

        return View{*this, m_data.get() + offset, passed_indices};
    }

/**
 * @brief Get a reference to a value in a tensor
 *
 * Special case of operator[] where all indices must be int.
 *
 * @return double& to a single value in the tensor.
 */
    template<impl::Indexable_c... Indices>
    requires (impl::AllInt_c<Indices...> && sizeof...(Indices) > 0)
    double& operator[](Indices... indices) {
        auto data = m_data.get();

        size_t n = 0;
        for(const auto index : {indices...}) {
            impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch");
            impl::deny(index < 0, "Indices cannot be less than zero");
            impl::deny(index >= m_dimensions[n].index.size(), "Index size mismatch");

            data += m_dimensions[n].width * index;
            ++n;
        }

        return *data;
    }

/**
 * @overload double& operator[](Indices... indices)
 */
    template<impl::Indexable_c... Indices>
    double& operator[](Indices...) requires (sizeof...(Indices) == 0) {
        return *m_data;
    }

/**
 * @brief Index a view with a vector of indexables
 */
///@{
    View operator[](impl::Indexables indices) const;
    double& operator[](const std::vector<int>& indices) const;
///@}

    [[nodiscard]] std::string name() const;
    [[nodiscard]] size_t size() const;
    [[nodiscard]] size_t size(int dimension) const;
    [[nodiscard]] int rank() const;
    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;
    [[nodiscard]] bool is_metric() const;
    [[nodiscard]] bool has_index(const Index& index) const;
    [[nodiscard]] size_t index_position(const Index& index) const;
    [[nodiscard]] std::vector<Index> indices() const;
    [[nodiscard]] const Index& indices(int index) const;
    [[nodiscard]] std::vector<VarianceQualifiedIndex> qualified_indices() const;
    [[nodiscard]] Variance variance(const Index& index) const;
    [[nodiscard]] Variance variance(int index) const;

/**
 * @brief Outputs a comma-separated dump of every value in the tensor
 */
    template<typename S>
    requires requires(S stream) {
        stream << std::string{""};
    }
    std::ostream& dump(S& ostream) const {
        // outputs a comma-separated dump of every value in the tensor
        const double* data = m_data.get();
        ostream << std::to_string(*data);
        for(size_t i=1; i<m_size; ++i) {
            ostream << std::string{", "} << std::to_string(*(data + i));
        }
        ostream << "\n";

        return ostream;
    }

/**
 * @brief Print the values of a tensor
 *
 * The tensor's values are displayed in a geometrically intuitive fashion up to rank 4. For higher rank tensors, the
 * values are just dumped as a list.
 */
    friend void write_data(std::ostream& stream, const Tensor& tensor);

/**
 * @brief Print a summary display of a tensor including name and indices
 *
 * Name and indices are displayed as a large title with the indices placed according to variance. The tensor's values
 * are displayed in a geometrically intuitive fashion up to rank 4. For higher rank tensors, the values are just dumped
 * as a list.
 */
    friend std::ostream& pretty_print(std::ostream& ostream, const Tensor& tensor);

/**
 * @brief Get the tensor's internal name (default to "Vari-Tensor")
 */
    Tensor& set_name(const std::string& name);
    /**
 * @brief Mathematically swap two indices and transpose the associated elements in memory
 */
    Tensor& transpose(const Index& first, const Index& second);
/**
 * @brief Replace one index with another
 */
    Tensor& relabel(const Index& old_index, const Index& new_index);
/**
 * @brief Directly set the variance of an index
 *
 * Does not manipulate the values of the tensor - for a true mathematical variance change, a tensor must be contracted
 * by multiplying with a metric tensor obtained from varitensor::metric_tensor(...).
 */
    Tensor& set_variance(const Index& index, Variance variance);
/**
 * @brief Alias for set_variance() with CONTRAVARIANT
 */
    Tensor& raise(const Index& index);
/**
 * @brief Alias for set_variance() with COVARIANT
 */
    Tensor& lower(const Index& index);

/**
 * @brief Standard iterator and const iterator functions
 */
///@{
    [[nodiscard]] View::iterator begin() const;
    [[nodiscard]] View::iterator end() const;
    [[nodiscard]] View::const_iterator cbegin() const;
    [[nodiscard]] View::const_iterator cend() const;
///@}

    impl::ProductOp operator-() const;

    friend impl::LinkedOp operator+(const Tensor& first, const Tensor& second);
    friend impl::LinkedOp operator+(const Tensor& first, const double& second);
    friend impl::LinkedOp operator+(const double& first, const Tensor& second);

    friend Tensor& operator+=(Tensor& first, const Tensor& second);
    friend double& operator+=(double& first, const Tensor& second);
    friend Tensor& operator+=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator+=(Tensor& first, Expression&& second) {
        (first + second).populate(first, false);
        return first;
    }

    friend impl::LinkedOp operator-(const Tensor& first, const Tensor& second);
    friend impl::LinkedOp operator-(const Tensor& first, const double& second);
    friend impl::LinkedOp operator-(const double& first, const Tensor& second);

    friend Tensor& operator-=(Tensor& first, const Tensor& second);
    friend double& operator-=(double& first, const Tensor& second);
    friend Tensor& operator-=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator-=(Tensor& first, Expression&& expression) {
        (first - expression).populate(first, false);
        return first;
    }

    friend impl::ProductOp operator*(const Tensor& first, const Tensor& second);
    friend impl::ProductOp operator*(const Tensor& first, const double& second);
    friend impl::ProductOp operator*(const double& first, const Tensor& second);

    friend Tensor& operator*=(Tensor& first, const Tensor& second);
    friend double& operator*=(double& first, const Tensor& second);
    friend Tensor& operator*=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator*=(Tensor& first, Expression&& second) {
        (first * second).populate(first);
        return first;
    }

    friend impl::ProductOp operator/(const Tensor& first, const Tensor& second);
    friend impl::ProductOp operator/(const Tensor& first, const double& second);
    friend impl::ProductOp operator/(const double& first, const Tensor& second);

    friend Tensor& operator/=(Tensor& first, const Tensor& second);
    friend double& operator/=(double& first, const Tensor& second);
    friend Tensor& operator/=(Tensor& first, const double& second);

    template<impl::Expression_c Expression>
    friend Tensor& operator/=(Tensor& first, Expression&& second) {
        (first / second).populate(first);
        return first;
    }

/**
 * @brief Check for equality
 *
 * Note that for tensor-tensor the comparison ignores name and whether or not the tensor is a metric tensor.
 */
///@{
    friend bool operator==(const Tensor& first, const Tensor& second);
    friend bool operator==(const Tensor& first, const double& second);
    friend bool operator==(const double& first, const Tensor& second);
///@}

private:
    friend class View;
    friend class impl::LinkedOp;
    friend class impl::ProductOp;

    impl::Dimensions m_dimensions;
    size_t m_size{1};
    impl::DoublePtr m_data{nullptr, [](double*){}};
    std::string m_name{impl::TENSOR_DEFAULT_NAME};
    impl::TensorClass m_tensor_class{impl::TENSOR};

    friend Tensor levi_civita_symbol(std::initializer_list<Index> indices);
    friend Tensor levi_civita_symbol(std::initializer_list<VarianceQualifiedIndex> indices);
    friend Tensor antisymmetric_symbol(std::initializer_list<Index> indices);
    friend Tensor antisymmetric_symbol(std::initializer_list<VarianceQualifiedIndex> indices);
    friend Tensor kronecker_delta(std::initializer_list<Index> indices);
    friend Tensor kronecker_delta(std::initializer_list<VarianceQualifiedIndex> indices);
    friend Tensor metric_tensor(std::initializer_list<VarianceQualifiedIndex>, const MetricFunction&);
    // unqualified indices don't make sense for a metric tensor

    template<impl::Indices_c I>
    static Tensor make_levi_civita_symbol(const I& indices) {
        impl::deny(indices.size() <= 1,
                        "Levi-Civita Symbol must have at least 2 indices");

        const auto expected_size = indices.begin()->index.size();
        for (const auto& [index, _]: indices) {
            impl::deny(index.size() != expected_size,
                            "Indices to Levi-Civita Symbol must all the the same size");
        }

        auto epsilon = Tensor{"Epsilon",  indices};
        epsilon.set_tensor_class(impl::LEVI_CIVITA_SYMBOL);

        std::vector<int> permutation(indices.size());
        std::ranges::iota(permutation, 0);
        do {
            int inversions = 0;
            for (size_t i=0; i<permutation.size(); ++i) {
                for (size_t j=0; j<permutation.size(); ++j) {
                    if (
                        (j < i && permutation[i] < permutation[j]) ||
                        (j > i && permutation[i] > permutation[j])
                    ) {
                        ++inversions;
                    }
                }
            }
            if (inversions / 2 % 2) epsilon[permutation] = -1;
            else epsilon[permutation] = 1;
        }
        while (std::ranges::next_permutation(permutation).found);

        return epsilon;
    }

    template<impl::Indices_c I>
    static Tensor make_kronecker_delta(const I& indices) {
        impl::deny(indices.size() <= 1,
                        "Kronecker Delta must have at least 2 indices");

        const auto expected_size = indices.begin()->index.size();
        for (const auto& [index, _]: indices) {
            impl::deny(index.size() != expected_size,
                            "Indices to Kronecker Delta must all the the same size");
        }

        auto delta = Tensor{"delta", indices};
        delta.set_tensor_class(impl::KRONECKER_DELTA);

        // The diagonal memory locations are multiples of the sum of the index widths; for a
        // contiguous, symmetric tensor, this sum is given by the geometric series:
        // sum[n=0 to rank](index_length^n).
        const auto index_length_sum = static_cast<size_t>(
            (std::pow(expected_size, delta.rank()) - 1) / (expected_size - 1)
        );

        double* data = delta.m_data.get();
        for (int i=0; i<expected_size; ++i) {
            data[i * index_length_sum] = 1;
        }

        return delta;
    }

    void set_tensor_class(impl::TensorClass tensor_class);

    template<impl::VarianceQualifiedIndices_c V>
    void from_variance_qualified_indices(const V& variance_qualified_indices) {
        for(auto& vqi: variance_qualified_indices) {
            for (auto& dimension: m_dimensions) {
                impl::deny(dimension.index.id() == vqi.index.id(),
                                "Cannot initialize tensor with repeated indices");
            }
            m_dimensions.emplace_back(vqi.index, vqi.variance, m_size);
            m_size *= vqi.index.size();
        }
    }

    template<typename IteratorType>
    void populate_general(IteratorType& iter, IteratorType end, const bool allocate) {
        if (allocate) {
            const size_t new_size = iter.size();
            impl::DoublePtr new_data = impl::allocate(new_size);

            double* running_ptr = new_data.get();
            for (; iter != end; ++iter) *running_ptr++ = iter.deref();

            m_dimensions = iter.dimensions();
            m_size = new_size;
            m_data.swap(new_data);
        }
        else {
            double* running_ptr = m_data.get();
            for (; iter != end; ++iter) *running_ptr++ = iter.deref();
        }
    }

    void populate_scalar(double scalar, bool allocate);

    template<impl::Indexable_c... Indices>
    void construct_passed_indices(
        size_t& n,
        size_t& offset,
        impl::Dimensions& passed_indices,
        const Index& index,
        Indices... indices
    ) const {
        impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch!");
        impl::deny(index.size() > m_dimensions[n].index.size(), "Index size mismatch");

        passed_indices.emplace_back(index, m_dimensions[n].variance, m_dimensions[n].width);
        construct_passed_indices(++n, offset, passed_indices, indices...);
    }

    template<impl::Indexable_c... Indices>
    void construct_passed_indices(
        size_t& n,
        size_t& offset,
        impl::Dimensions& passed_indices,
        const Interval& index,
        Indices... indices
    ) const {
        impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch!");
        impl::deny(index.last >= m_dimensions[n].index.size(), "Index size mismatch");

        offset += m_dimensions[n].width * index.first;
        passed_indices.emplace_back(index, m_dimensions[n].variance, m_dimensions[n].width);
        construct_passed_indices(++n, offset, passed_indices, indices...);
    }

    template<impl::Indexable_c... Indices>
    void construct_passed_indices(
        size_t& n,
        size_t& offset,
        impl::Dimensions& passed_indices,
        const int index,
        Indices... indices
    ) const {
        impl::deny(n >= m_dimensions.size(), "Indexing dimension mismatch!");
        impl::deny(index+1 > m_dimensions[n].index.size(), "Index size mismatch");

        offset += index * m_dimensions[n].width;
        construct_passed_indices(++n, offset, passed_indices, indices...);
    }

    static void construct_passed_indices(size_t&, size_t&, impl::Dimensions&) {} // terminating overload
};

namespace impl {

template<>
inline bool ViewIterator<>::is_metric() const {
    return m_target->is_metric();
}

template<>
inline bool ViewIterator<true>::is_metric() const {
    return m_target->is_metric();
}

struct GetTensor{
    template <typename E>
    Tensor operator()(E&& expression) {
        return Tensor{expression};
    }
};

#if defined (__AVX__)

#include <immintrin.h>

constexpr size_t REG_WIDTH_256 = 4;
constexpr std::align_val_t M256_ALIGN{alignof(__m256d)};

inline void deallocate(double* data) {
    operator delete[](data, M256_ALIGN);
}

inline DoublePtr allocate(const size_t size) {
    const size_t remainder = size % REG_WIDTH_256;
    const size_t padded_size = remainder ? size - remainder + REG_WIDTH_256 : size;

    return DoublePtr{
        static_cast<double*> (
            operator new[](sizeof(double) * padded_size, M256_ALIGN)
        ),
        deallocate
    };
}

inline DoublePtr allocate_zeroed(const size_t size) {
    DoublePtr data = allocate(size);
    std::memset(data.get(), 0, size*sizeof(double));
    return data;
}

inline void copy(double* data1, const double* data2, const size_t size) {
    size_t i{0};
    for (; i<size - size%REG_WIDTH_256; i+=REG_WIDTH_256) {
        _mm256_store_pd(
            data1 + i,
            _mm256_loadu_pd(data2 + i)
        );
    }
    for (; i<size; ++i) data1[i] = data2[i];
}

inline void broadcast_vec(const double* data1, double* data2, const size_t size1, const size_t size2) {
    size_t j_start{0};
    for (int i=0; i < static_cast<int>(size1 - REG_WIDTH_256 + 1); i += REG_WIDTH_256) {
        const __m256d values = _mm256_load_pd(data1 + i);
        for (size_t j=j_start; j<size2; j+=size1) {
            _mm256_storeu_pd(data2 + j, values);
        }
        j_start += REG_WIDTH_256;
    }

    for (size_t i=size1 - size1 % REG_WIDTH_256; i<size1; ++i) {
        const double value = data1[i];
        for (size_t j = j_start; j<size2; j+=size1) {
            data2[j] = value;
        }
        ++j_start;
    }
}

inline void broadcast_chunks(const double* data1, double* data2, const size_t size1, const size_t interval) {
    double* running_ptr = data2;
    for (size_t i=0; i < size1; ++i) {
        const __m256d values = _mm256_set1_pd(data1[i]);
        for (int j=0; j < static_cast<int>(interval - REG_WIDTH_256 + 1); j += REG_WIDTH_256) {
            _mm256_storeu_pd(
                running_ptr,
                _mm256_mul_pd(
                    values,
                    _mm256_loadu_pd(running_ptr)
                )
            );

            running_ptr += REG_WIDTH_256;
        }
        for (size_t j=0; j < interval % REG_WIDTH_256; ++j) {
            *running_ptr *= data1[i];
            ++running_ptr;
        }
    }
}

inline void broadcast(const double value, double* data, const size_t size) {
    const __m256d scalar_vec = _mm256_set1_pd(value);
    for (size_t i=0; i<size; i+=REG_WIDTH_256) {
        _mm256_store_pd(
            data + i,
            _mm256_mul_pd(
                _mm256_load_pd(data + i),
                scalar_vec
            )
        );
    }
}

inline void piecewise(const double* data1, const double* data2,  double* data3, const size_t size, const Operation operation) {
    switch (operation) {
    case ADD:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_add_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    case SUB:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_sub_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    case MUL:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_mul_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    case DIV:
        for (size_t i=0; i<size; i+=REG_WIDTH_256) {
            _mm256_store_pd(data3 + i,  _mm256_div_pd(_mm256_load_pd(data1 + i), _mm256_load_pd(data2 + i)));
        }
        break;
    }
}

#else

inline void deallocate(double* data) {
    std::free(data);
}

inline DoublePtr allocate(const size_t size) {
    auto data = DoublePtr{
        static_cast<double*>(malloc(size * sizeof(double))),
        deallocate
    };
    if (!data) throw std::bad_alloc{};
    return data;
}

inline DoublePtr allocate_zeroed(const size_t size) {
    return DoublePtr{
        static_cast<double*>(calloc(size, sizeof(double))),
        deallocate
    };
}

inline void copy(double* data1, const double* data2, const size_t size) {
    for (size_t i=0; i<size; ++i) data1[i] = data2[i];
}

inline void broadcast_vec(const double* data1, double* data2, const size_t size1, const size_t size2) {
    for (size_t i=0; i < size1; i++) {
        const double current_value = data1[i];
        for (size_t j=i; j < size2; j += size1) {
            data2[j] = current_value;
        }
    }
}

inline void broadcast_chunks(const double* data1, double* data2, const size_t size1, const size_t interval) {
    for (size_t i=0; i < size1; ++i) {
        const double current_value = data1[i];
        for (size_t j=0; j < interval; j++) {
            *data2 *= current_value;
            ++data2;
        }
    }
}

inline void broadcast(const double value, double* data, const size_t size) {
    for (size_t i=0; i<size; ++i) data[i] *= value;
}

inline void piecewise(const double* data1, const double* data2,  double* data3, const size_t size, const Operation operation) {
    switch (operation) {
    case ADD: for (size_t i=0; i<size; ++i) data3[i] = data1[i] + data2[i]; break;
    case SUB: for (size_t i=0; i<size; ++i) data3[i] = data1[i] - data2[i]; break;
    case MUL: for (size_t i=0; i<size; ++i) data3[i] = data1[i] * data2[i]; break;
    case DIV: for (size_t i=0; i<size; ++i) data3[i] = data1[i] / data2[i]; break;
    }
}

#endif

inline DoublePtr allocate(const size_t size, const double& initial_value) {
    DoublePtr data = allocate(size);
    std::fill_n(data.get(), size, initial_value);
    return data;
}

inline DoublePtr allocate_copy(const double* data, const size_t size) {
    DoublePtr copy = allocate(size);
    std::memcpy(copy.get(), data, size * sizeof(double));
    return copy;
}

struct VariTensorInternalError final: std::logic_error {
    explicit VariTensorInternalError(const std::string& message):
        std::logic_error("<Vari-Tensor> Tensor logic error: " + message)
    {}
};

/**
 * @brief Increments the positions vector in accordance with the dimensions vector
 *
 * The iterator provided will be told which index to increment, or reset if a dimension is overflown. Returns true if
 * the increment was successful, false if the end of the dimensions has been reached.
 */
template <ExpressionIterator_c E>
bool increment_positions( // static to avoid having to use virtual functions
    std::vector<int>& positions,
    const std::vector<Dimension>& dimensions,
    const E& iterator // not really const, but we have to pretend to maintain the constness of the * operators
) {
    for (size_t i=0; i<positions.size(); ++i) {
        // check if we're about to overflow the next index
        if (positions[i] + 1 == dimensions[i].index.size()) [[unlikely]] {
            // if so, reset it and move on to the next one
            positions[i] = 0;
            iterator.reset(dimensions[i].index.id());
            continue;
        }

        // once we've found an index we can increment, do so
        ++positions[i];
        iterator.increment(dimensions[i].index.id());

        return true;
    }

    // note that if we reach the end, all the indices will have been reset to 0
    return false;
}

} // namespace impl

namespace impl {

class LinkedOpIterator: public ExpressionIteratorBase {
public:
    LinkedOpIterator() = default;
    LinkedOpIterator(double modifier, Preparatory& preparatory, const std::vector<Operation>* signs, bool end=false);
    LinkedOpIterator(const LinkedOpIterator& other) = default;
    LinkedOpIterator& operator=(const LinkedOpIterator& other) = default;
    LinkedOpIterator(LinkedOpIterator&& other) noexcept = default;
    LinkedOpIterator& operator=(LinkedOpIterator&& other) = default;

    LinkedOpIterator& operator++();
    LinkedOpIterator operator++(int);

    double operator*() const;

    bool operator==(const LinkedOpIterator& other) const;

    [[nodiscard]] static bool is_metric();
    [[nodiscard]] static bool is_contiguous();
    [[nodiscard]] static double* data();

    void increment(int index_id) const;
    void reset(int index_id) const;

    [[nodiscard]] double deref() const;

private:
    double m_modifier{0};
    ExpressionIterators m_sub_iterators{};
    const std::vector<Operation>* m_signs{nullptr};  // * to allow default construction

    bool m_end{false};
};

class ProductOpIterator: public ExpressionIteratorBase {
public:
    ProductOpIterator() = default;
    ProductOpIterator(double modifier, Preparatory& preparatory, bool end=false);
    ProductOpIterator(const ProductOpIterator& other) = default;
    ProductOpIterator& operator=(const ProductOpIterator& other) = default;
    ProductOpIterator(ProductOpIterator&& other) noexcept = default;
    ProductOpIterator& operator=(ProductOpIterator&& other) = default;

    ProductOpIterator& operator++();
    ProductOpIterator operator++(int);

    double operator*() const;

    bool operator==(const ProductOpIterator& other) const;

    [[nodiscard]] static bool is_metric();
    [[nodiscard]] static bool is_contiguous();
    [[nodiscard]] static double* data();

    void increment(const int index_id) const;
    void reset(const int index_id) const;

    [[nodiscard]] double deref() const;

private:
    double m_modifier{1};
    ExpressionIterators m_sub_iterators{};

    // for index summation
    Dimensions m_repeated;
    mutable std::vector<int> m_repeated_positions; // mutable for deref()

    bool m_end{false};
};

inline auto Deref = [](const auto& expression) {return expression.deref();};
inline auto IsContiguous = [](const auto& iter) {return iter.is_contiguous();};

class LinkedOp {
public:
    template <ExpressionOperand_c OperandType1, ExpressionOperand_c OperandType2>
    LinkedOp(const OperandType1& first, const OperandType2& second, Operation sign) {
        call_add_function(first);
        call_add_function(second, sign);
    }

    void populate(Tensor& tensor, bool allocate = true);

    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;

    using iterator = LinkedOpIterator;
    [[nodiscard]] iterator begin() const;
    [[nodiscard]] iterator end() const;
    [[nodiscard]] ExpressionIterator vbegin() const;

private:
    double m_modifier{0}; // used to collate double/scalar operands
    Expressions m_sub_expressions{};
    std::vector<Operation> m_signs;  // used to track the sign of each sub-expression

    template<ExpressionOperand_c OperandType>
    void call_add_function(const OperandType& operand, const Operation sign=ADD) {
        if constexpr (!std::is_same_v<OperandType, double>) {
            if (operand.is_scalar()) {
                // If we have a scalar Tensor or Expression, we can short-circuit by getting the value
                add_element(operand.get_scalar(), sign);
                return;
            }
        }
        add_element(operand, sign);
    }

    void add_element(double value, Operation sign);
    void add_element(const Tensor& tensor, Operation sign);
    void add_element(const LinkedOp& linked_op, Operation sign);
    void add_element(const ProductOp& product_op, Operation sign);

    void add_element(const View& view, Operation sign);
};

class ProductOp {
public:
    using iterator = ProductOpIterator;

    template <ExpressionOperand_c OperandType1, ExpressionOperand_c OperandType2>
    ProductOp(const OperandType1& first, const OperandType2& second, Operation operation)
    {
        // order matters - indices need to be added in the order they arrive
        call_add_function(first);
        call_add_function(second, operation);
    }

    void populate(Tensor& tensor);

    [[nodiscard]] bool is_scalar() const;
    [[nodiscard]] double get_scalar() const;

    [[nodiscard]] iterator begin() const;
    [[nodiscard]] iterator end() const;
    [[nodiscard]] ExpressionIterator vbegin() const;

private:
    double m_modifier{1}; // used to collate double/scalar operands
    Expressions m_sub_expressions{};

    template<ExpressionOperand_c OperandType>
    void call_add_function(const OperandType& operand, const Operation operation=MUL) {
        if constexpr (!std::is_same_v<OperandType, double>) {
            if (operand.is_scalar()) {
                // If we have a scalar Tensor or Expression, we can short-circuit by getting the value
                add_element(operation == MUL ? operand.get_scalar() : 1/operand.get_scalar());
            }
            else {
                deny(operation == DIV, "Cannot divide by non-scalar Tensor/Expression");
                add_element(operand);
            }
        }
        else {
            add_element(operation == MUL ? operand : 1/operand);
        }
    }

    void add_element(const double& value);
    void add_element(const Tensor& tensor);
    void add_element(const LinkedOp& linked_op);
    void add_element(const ProductOp& product_op);
    void add_element(const View& view);
};

} // namespace impl

/**
 * @brief Basic Euclidean metric function, used as the default for metric_tensor(...)
 */
const MetricFunction EUCLIDEAN_METRIC = [](const int i, const int j) {
    return i == j ? 1 : 0;
};

/**
 * @brief Creates a metric tensor that can be used to raise and lower indices
 *
 * Metric tensors function like normal tensors, but when used in einstein summation, their non-summed index positionally
 * replaces the summed one in the new tensor. This is commonly used in differential geometry to raise and lower indices.
 *
 * e.g. B_i_j * g^i^k = B^k_j
 *
 * This tensor will always use a lowercase g symbol when pretty printing.
 *
 * @param indices The indices with which to initialise the metric. There must be two indices, both of the same length.
 *
 * @param metric_function A user-defined function to populate the metric. The function must accept two ints giving
 * the position in the metric and return a double giving the value; varitensor::MetricFunction is provided as a
 * helper alias.
 */
inline Tensor metric_tensor(
    const std::initializer_list<VarianceQualifiedIndex> indices,
    const MetricFunction& metric_function = EUCLIDEAN_METRIC
) {
    impl::deny(indices.size() != 2,
                    "Metric tensor must have exactly 2 indices");
    impl::deny(indices.begin()->index == (indices.begin() + 1)->index,
                    "Metric tensor indices must be different");
    impl::deny(indices.begin()->variance != (indices.begin() + 1)->variance,
                    "Metric tensor indices must have the same variance");

    Tensor metric{"g", indices};
    metric.set_tensor_class(impl::METRIC_TENSOR);

    const auto index_size = indices.begin()->index.size();
    for (int i = 0; i < index_size; ++i) {
        for (int j = 0; j < index_size; ++j) {
            metric[i, j] = metric_function(i, j);
        }
    }

    return metric;
}

/**
 * @brief Creates an n-dimensional Levi-Civita symbol
 *
 * This tensor will always use an epsilon symbol when pretty-printing.
 *
 * @param indices The indices with which to initialise the symbol. There must be at least 2 indices, and they must all
 * be the same length.
 */
///@{
inline Tensor levi_civita_symbol(const std::initializer_list<Index> indices) {
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    return Tensor::make_levi_civita_symbol(vq_indices);
}

inline Tensor levi_civita_symbol(const std::initializer_list<VarianceQualifiedIndex> indices) {
    return Tensor::make_levi_civita_symbol(indices);
}

inline Tensor antisymmetric_symbol(const std::initializer_list<Index> indices) {
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    return Tensor::make_levi_civita_symbol(vq_indices);
}

inline Tensor antisymmetric_symbol(const std::initializer_list<VarianceQualifiedIndex> indices) {
    return Tensor::make_levi_civita_symbol(indices);
}
///@}

/**
 * @brief Creates an n-dimensional Kronecker delta
 *
 * This tensor will always use an delta symbol when pretty-printing.
 *
 * @param indices The indices with which to initialise the delta. There must be at least 2 indices, and they must all
 * be the same length.
 */
///@{
inline Tensor kronecker_delta(const std::initializer_list<Index> indices) {
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    return Tensor::make_kronecker_delta(vq_indices);
}

inline Tensor kronecker_delta(const std::initializer_list<VarianceQualifiedIndex> indices) {
    return Tensor::make_kronecker_delta(indices);
}
///@}

// =================================================================================================
//                                                                                Preparatory impl |
// =================================================================================================

namespace impl {
inline Preparatory::Preparatory(const Expressions& sub_expressions, const PreparatoryType type) {
    if (sub_expressions.empty()) return; // scalar case

    if (type == LINKED) prepare_linked_operation(sub_expressions);
    else prepare_product_operation(sub_expressions);
}

inline void Preparatory::prepare_linked_operation(const Expressions& sub_expressions) {
    state = ALIGNED_INDICES;

    // construct the first iterator so that we have something to compare against
    sub_iterators.emplace_back(std::visit(VBegin, sub_expressions[0]));
    dimensions = std::visit(GetDimensions, sub_iterators[0]);
    size = std::visit(GetSize, sub_iterators[0]);

    std::function<void(unsigned&, const Dimensions&)> index_matching_function;

    auto find_index = [this] (
        const unsigned& j, const Dimensions& current_dimensions, unsigned k=0
    ) {
        for (; k<current_dimensions.size(); ++k) {
            if (dimensions[j].index == current_dimensions[k].index) {
                deny(dimensions[j].variance != current_dimensions[k].variance,
                    "Cannot add or subtract tensors with indices of disagreeing variance");
                return;
            }
        }
        deny(true, // this is always an error, but go through deny() for consistency
            "Cannot add or subtract tensors with un-pairable indices");
    };

    auto assume_aligned = [this, &find_index, &index_matching_function](
        const unsigned& j, const Dimensions& current_dimensions
    ) {
        deny(dimensions[j].variance != current_dimensions[j].variance,
                "Cannot add or subtract tensors with indices of disagreeing variance");
        if (dimensions[j].index != current_dimensions[j].index) {
            state = GENERAL;
            find_index(j, current_dimensions, j);
            index_matching_function = find_index; // switch to the general case
        }
    };

    // assume that we have only views with aligned indices, e.g. (T_ab + U_ab) NOT (T_ab + U_ba)
    index_matching_function = assume_aligned;
    for (unsigned i=1; i<sub_expressions.size(); ++i) { // start i at 1 as we just did the first iterator
        sub_iterators.emplace_back(std::visit(VBegin, sub_expressions[i]));
        deny(std::visit(GetDimensions, sub_iterators[i]).size() != dimensions.size(),
            "Cannot add or subtract tensors with different numbers of indices");

        const Dimensions& current_dimensions = std::visit(GetDimensions, sub_iterators[i]);
        for (unsigned j=0; j<dimensions.size(); ++j) {
            index_matching_function(j, current_dimensions);
        }
    }
}

inline void Preparatory::prepare_product_operation(const Expressions& sub_expressions) {
    state = FREE_MULTIPLICATION;

    for (auto& expression: sub_expressions) sub_iterators.emplace_back(std::visit(VBegin, expression));

    // get the naive dimensions whilst recording metric and repetition information that will be useful later
    Dimensions total_dimensions;
    std::map<int, Couple> partners;
    int i{0};

    for (auto& iterator: sub_iterators) {
        auto sub_dimensions = std::visit(GetDimensions, iterator);

        for (size_t k = 0; k < sub_dimensions.size(); ++k) {
            total_dimensions.emplace_back(sub_dimensions[k]);

            if (std::visit([](auto& expression) {return expression.is_metric();}, iterator)) {
                partners[sub_dimensions[k].index.id()].add(i, true);
                const int other = k == 0 ? i+1 : i-1; // metric tensors only have 2 indices
                partners[sub_dimensions[k].index.id()].metric_index = other;
            }
            else {
                partners[sub_dimensions[k].index.id()].add(i, false);
            }

            ++i;
        }
    }

    // handle any metric indices
    for (auto& couple: partners | std::views::values) {
        if (couple.is_repeated() && couple.is_metric()) {
            std::swap(total_dimensions[couple.metric_index], total_dimensions[couple.other_index]);
        }
    }

    for (auto& dimension: total_dimensions) {
        if (auto& couple = partners[dimension.index.id()]; couple.is_repeated()) { // repeated index
            repeated.emplace_back(dimension);
            couple.clear();
            state = GENERAL;
        }
        else if (couple.has()) { // non-repeated index
            dimensions.push_back(dimension);
            dimensions.back().width = size;
            size *= dimension.size();
        }
    }
}

} // namespace impl

// =================================================================================================
//                                                                                       View impl |
// =================================================================================================

inline View::View(const Tensor& target):
    m_target{target},
    m_data_ptr{target.m_data.get()},
    m_dimensions{m_target.m_dimensions}
{
    for (size_t i = 0; i < m_dimensions.size(); ++i) {
        impl::deny(m_dimensions[i].size() > m_target.m_dimensions[i].index.size(),
                        "Index size mismatch");
        impl::deny(m_dimensions[i].variance != m_target.m_dimensions[i].variance,
                        "Index variance mismatch");
    }
}

inline View::View(const Tensor& target, double* const data_ptr, impl::Dimensions dimensions):
    m_target{target},
    m_data_ptr{data_ptr},
    m_dimensions{std::move(dimensions)}
{} // we only call this from Tensor::operator[], so no need to validate the dimensions

void View::operator=(const Tensor& other) && { // NOLINT - allows T[i, 2] = U
    const View other_view{other};

    impl::deny(m_dimensions.size() != other.m_dimensions.size(),
                    "Cannot assign to View with different dimensions");
    for (size_t i = 0; i < m_dimensions.size(); ++i) {
        impl::deny(m_dimensions[i].index.size() != other.m_dimensions[i].index.size(),
                        "Index size mismatch when assigning view");
        impl::deny(m_dimensions[i].variance != other.m_dimensions[i].variance,
                        "Variance mismatch when assigning view");
    }

    auto iter1 = begin();
    auto iter2 = other_view.begin();
    impl::deny(iter1.is_contracted(), "Cannot assign to a contraction expression");

    for (auto end=this->end(); iter1 != end; ++iter1, ++iter2) *iter1 = *iter2;
}

inline bool View::operator==(const View& other) const {
    auto iter1 = begin();
    auto iter2 = other.begin();
    for (auto end = this->end(); iter1 != end;  ++iter1, ++iter2) {
        if (*iter1 != *iter2) return false;
    }
    return iter2.finished();
}

inline View::iterator View::begin() {
    return iterator{&m_target, m_data_ptr, m_dimensions};
}

inline View::iterator View::end() {
    return iterator{&m_target, nullptr, {}};
}

inline View::const_iterator View::begin() const {
    return const_iterator{&m_target, m_data_ptr, m_dimensions};
}

inline View::const_iterator View::end() const {
    return const_iterator{&m_target, nullptr, {}};
}

inline View::const_iterator View::cbegin() const {
    return const_iterator{&m_target, m_data_ptr, m_dimensions};
}

inline View::const_iterator View::cend() const {
    return const_iterator{&m_target, nullptr, {}};
}

inline impl::ExpressionIterator View::vbegin() const {
    return begin();
}

[[nodiscard]] inline double* View::data() const {
    return m_data_ptr;
}

[[nodiscard]] inline bool View::is_scalar() const {
    return m_dimensions.empty();
}

[[nodiscard]] inline double View::get_scalar() const {
    return *m_data_ptr;
}

inline void View::populate(Tensor& tensor, const bool allocate/* = true */) const {
    auto iter = cbegin();

    if (allocate) {
        tensor.m_dimensions = iter.dimensions();
        tensor.m_size = iter.size();
        tensor.m_data = impl::allocate(tensor.m_size);
    }

    if (!iter.is_contracted() && iter.is_contiguous()) {
        impl::copy(tensor.m_data.get(), m_data_ptr, tensor.m_size);
    }
    else {
        double* running_ptr = tensor.m_data.get();
        for (const auto end=cend(); iter != end; ++iter, ++running_ptr) {
            *running_ptr = *iter;
        }
    }
}

// =================================================================================================
//                                                                           LinkedOpIterator impl |
// =================================================================================================
namespace impl {

[[nodiscard]] inline double* LinkedOpIterator::data() {
    throw VariTensorInternalError("Function should never be called");
}

inline LinkedOpIterator::LinkedOpIterator(
    const double modifier,
    Preparatory& preparatory,
    const std::vector<Operation>* signs,
    const bool end/* = false */
):
    ExpressionIteratorBase{preparatory},
    m_modifier{modifier},
    m_sub_iterators{std::move(preparatory.sub_iterators)},
    m_signs{signs},
    m_end{end}
{}

inline LinkedOpIterator& LinkedOpIterator::operator++() {
    m_end = !increment_positions(m_positions, m_dimensions, *this);
    return *this;
}

inline LinkedOpIterator LinkedOpIterator::operator++(int) {
    LinkedOpIterator copy = *this;
    ++*this;
    return copy;
}

inline double LinkedOpIterator::operator*() const {
    return deref();
}

inline bool LinkedOpIterator::operator==(const LinkedOpIterator& other) const {
    if (m_end != other.m_end) [[likely]] return false;
    if (m_end) return true;
    return m_modifier == other.m_modifier && m_positions == other.m_positions; // good enough for normal iteration
}

inline void LinkedOpIterator::increment(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Increment(index_id), iterator);
}

inline void LinkedOpIterator::reset(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Reset(index_id), iterator);
}

inline double LinkedOpIterator::deref() const {
    double sum = 0;
    for (size_t i=0; i < m_sub_iterators.size(); ++i) {
        sum += std::visit(Deref, m_sub_iterators[i]) * static_cast<double> ((*m_signs)[i]);
    }
    return sum + m_modifier;
}

inline bool LinkedOpIterator::is_metric() {
    return false;
}

inline bool LinkedOpIterator::is_contiguous() {
    return false;
}

// =================================================================================================
//                                                                          ProductOpIterator impl |
// =================================================================================================

[[nodiscard]] inline double* ProductOpIterator::data() {
    throw VariTensorInternalError("Function should never be called");
}

inline ProductOpIterator::ProductOpIterator(
    const double modifier,
    Preparatory& preparatory,
    const bool end/* = false */
):
    ExpressionIteratorBase{preparatory},
    m_modifier{modifier},
    m_sub_iterators{std::move(preparatory.sub_iterators)},
    m_repeated{std::move(preparatory.repeated)},
    m_repeated_positions(m_repeated.size(), 0),
    m_end{end}
{}

inline ProductOpIterator& ProductOpIterator::operator++() {
    m_end = !increment_positions(m_positions, m_dimensions, *this);
    return *this;
}

inline ProductOpIterator ProductOpIterator::operator++(int) {
    ProductOpIterator copy = *this;
    ++*this;
    return copy;
}

inline double ProductOpIterator::operator*() const {
    return deref();
}

inline bool ProductOpIterator::operator==(const ProductOpIterator& other) const {
    if (m_end != other.m_end) [[likely]] return false;
    if (m_end) return true;
    return m_modifier == other.m_modifier && m_positions == other.m_positions; // good enough for normal iteration
}

inline void ProductOpIterator::increment(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Increment(index_id), iterator);
}

inline void ProductOpIterator::reset(const int index_id) const {
    for (auto& iterator: m_sub_iterators) std::visit(Reset(index_id), iterator);
}

inline double ProductOpIterator::deref() const {
    /* Despite modifying m_repeated_positions, this function is const as it always returns the positions to 0 */
    double sum = 0;

    do {
        double product = 1;
        for (auto& iterator: m_sub_iterators) product *= std::visit(Deref, iterator);
        sum += product;
    }
    while (increment_positions(m_repeated_positions, m_repeated, *this));

    return sum * m_modifier;
}

inline bool ProductOpIterator::is_metric() {
    return false;
}

inline bool ProductOpIterator::is_contiguous() {
    return false;
}

// =================================================================================================
//                                                                                   LinkedOp impl |
// =================================================================================================

inline LinkedOpIterator LinkedOp::begin() const {
    Preparatory preparatory{m_sub_expressions, LINKED};
    return iterator{m_modifier, preparatory, &m_signs};
}

inline LinkedOpIterator LinkedOp::end() const {
    Preparatory preparatory{};
    return iterator{m_modifier, preparatory, &m_signs, true};
}

inline ExpressionIterator LinkedOp::vbegin() const {
    return begin();
}

[[nodiscard]] inline bool LinkedOp::is_scalar() const {
    return m_sub_expressions.empty();
}

[[nodiscard]] inline double LinkedOp::get_scalar() const {
    return m_modifier;
}

inline void LinkedOp::add_element(const double value, const Operation sign) {
    deny(!is_scalar(), "Cannot add/subtract scalar with non-scalar expression");
    sign == ADD ? m_modifier += value : m_modifier -= value;
}

inline void LinkedOp::add_element(const Tensor& tensor, const Operation sign) {
    m_sub_expressions.emplace_back(View{tensor});
    sign == ADD ? m_signs.push_back(ADD) : m_signs.push_back(SUB);
}

inline void LinkedOp::add_element(const LinkedOp& linked_op, const Operation sign) {
    if (sign == ADD) {
        m_modifier += linked_op.m_modifier;
        m_signs.append_range(linked_op.m_signs);
    }
    else {
        m_modifier -= linked_op.m_modifier;
        for (auto& sub_sign: linked_op.m_signs) m_signs.push_back(static_cast<Operation>(sub_sign * -1));
    }
    m_sub_expressions.append_range(linked_op.m_sub_expressions);
}

inline void LinkedOp::add_element(const ProductOp& product_op, const Operation sign) {
    m_sub_expressions.emplace_back(product_op);
    m_signs.push_back(sign);
}

inline void LinkedOp::add_element(const View& view, const Operation sign) {
    m_sub_expressions.emplace_back(view);
    m_signs.push_back(sign);
}

inline void LinkedOp::populate(Tensor& tensor, const bool allocate/* = true */) {
    auto populate_aligned_indices = [&](Preparatory& preparatory) {
        if (allocate) {
            tensor.m_dimensions = preparatory.dimensions;
            tensor.m_size = preparatory.size;
            DoublePtr new_data = allocate_copy(
                std::visit(IsContiguous, preparatory.sub_iterators[0]) ?
                    std::get<View>(m_sub_expressions[0]).data() :
                    std::visit(GetTensor{}, m_sub_expressions[0]).m_data.get(),
                tensor.m_size
            );
            tensor.m_data.swap(new_data);
        }

        // for +=/-=, we start with the 0th in the right place, for +/- we handled with 0th above, so start i at 1
        for (unsigned i=1; i<preparatory.sub_iterators.size(); ++i) {
            piecewise(
                tensor.m_data.get(),
                std::visit(IsContiguous, preparatory.sub_iterators[i]) ?
                    std::get<View>(m_sub_expressions[i]).data() :
                    std::visit(GetTensor{}, m_sub_expressions[i]).m_data.get(),
                tensor.m_data.get(),
                tensor.m_size,
                m_signs[i]
            );
        }
    };

    switch (Preparatory preparatory{m_sub_expressions, LINKED}; preparatory.state) {
    case SCALAR:
        tensor.populate_scalar(m_modifier, allocate);
        break;
    case ALIGNED_INDICES:
        populate_aligned_indices(preparatory);
        break;
    case GENERAL: {
        iterator iter{m_modifier, preparatory, &m_signs};
        tensor.populate_general(iter, end(), allocate);
        break;
    }
    case FREE_MULTIPLICATION: // to stop the compiler warning
    default:
        throw VariTensorInternalError("Invalid expression state!");
    }
}

// =================================================================================================
//                                                                                  ProductOp impl |
// =================================================================================================

inline void ProductOp::populate(Tensor& tensor) {
    auto populate_free_multiplication = [&](Preparatory& preparatory) {
        DoublePtr new_data = allocate(preparatory.size, 0);

        if (std::visit(IsContiguous, preparatory.sub_iterators[0])) {
            broadcast_vec(
                std::visit(GetData, preparatory.sub_iterators[0]),
                new_data.get(),
                std::visit(GetSize, preparatory.sub_iterators[0]),
                preparatory.size
            );
        }
        else {
            const auto tmp = std::visit(GetTensor{}, m_sub_expressions[0]);
            broadcast_vec(tmp.m_data.get(), new_data.get(), tmp.size(), preparatory.size);
        }

        size_t dim_i = std::visit(GetDimensions, preparatory.sub_iterators[0]).size() - 1;
        size_t width = std::visit(GetSize, preparatory.sub_iterators[0]);
        for (unsigned i=1; i < preparatory.sub_iterators.size(); ++i) {
            const size_t dim_i_previous = dim_i;
            dim_i += std::visit(GetDimensions, preparatory.sub_iterators[i]).size();

            if (std::visit(IsContiguous, preparatory.sub_iterators[i])) {
                broadcast_chunks(
                    std::visit(GetData, preparatory.sub_iterators[i]),
                    new_data.get(),
                    std::visit(GetSize, preparatory.sub_iterators[i]),
                    width
                );
            }
            else {
                const auto tmp = std::visit(GetTensor{}, m_sub_expressions[i]);
                broadcast_chunks(tmp.m_data.get(), new_data.get(), tmp.size(), width);
            }

            for (size_t j=dim_i_previous; j < dim_i; ++j) width *= preparatory.dimensions[j].size();
        }

        broadcast(m_modifier, new_data.get(), preparatory.size);

        // Set up the tensor at the end in case we're doing an assignment
        tensor.m_dimensions = preparatory.dimensions;
        tensor.m_size = preparatory.size;
        tensor.m_data.swap(new_data);
    };

    switch (Preparatory preparatory{m_sub_expressions, PRODUCT}; preparatory.state) {
    case SCALAR:
        tensor.populate_scalar(m_modifier, true);
        break;
    case FREE_MULTIPLICATION:
        populate_free_multiplication(preparatory);
        break;
    case GENERAL: {
        iterator iter{m_modifier, preparatory};
        tensor.populate_general(iter, end(), true);
        break;
    }
    default:
        throw VariTensorInternalError("Invalid expression state!");
    }
}

inline ProductOp::iterator ProductOp::begin() const {
    Preparatory preparatory{m_sub_expressions, PRODUCT};
    return iterator{m_modifier, preparatory};
}

inline ProductOp::iterator ProductOp::end() const {
    Preparatory preparatory{};
    return iterator{m_modifier, preparatory, true};
}

inline ExpressionIterator ProductOp::vbegin() const {
    return begin();
}

[[nodiscard]] inline bool ProductOp::is_scalar() const {
    return m_sub_expressions.empty();
}

[[nodiscard]] inline double ProductOp::get_scalar() const {
    return m_modifier;
}

inline void ProductOp::add_element(const double& value) {
    m_modifier *= value;
}

inline void ProductOp::add_element(const Tensor& tensor) {
    m_sub_expressions.emplace_back(View{tensor});
}

inline void ProductOp::add_element(const LinkedOp& linked_op) {
    m_sub_expressions.emplace_back(linked_op);
}

inline void ProductOp::add_element(const ProductOp& product_op) {
    m_modifier *= product_op.m_modifier;
    for (auto& expression: product_op.m_sub_expressions) m_sub_expressions.emplace_back(expression);
}

inline void ProductOp::add_element(const View& view) {
    m_sub_expressions.emplace_back(view);
}

} // namespace impl

// =================================================================================================
//                                                                                     Tensor impl |
// =================================================================================================

inline Tensor::Tensor(const std::initializer_list<VarianceQualifiedIndex> vq_indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices}
{}

inline Tensor::Tensor(const std::initializer_list<VarianceQualifiedIndex> vq_indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::initializer_list<VarianceQualifiedIndex> vq_indices):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);
}

inline Tensor::Tensor(std::string name, const std::initializer_list<VarianceQualifiedIndex> vq_indices, const double initial_value):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const std::initializer_list<Index> indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices}
{}

inline Tensor::Tensor(const std::initializer_list<Index> indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::initializer_list<Index> indices):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);

}

inline Tensor::Tensor(std::string name, const std::initializer_list<Index> indices, const double initial_value):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices}
{}

inline Tensor::Tensor(const std::vector<VarianceQualifiedIndex>& vq_indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, vq_indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);
}

inline Tensor::Tensor(std::string name, const std::vector<VarianceQualifiedIndex>& vq_indices, const double initial_value):
    m_name{std::move(name)}
{
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const std::vector<Index>& indices):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices}
{}

inline Tensor::Tensor(const std::vector<Index>& indices, const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, indices, initial_value}
{}

inline Tensor::Tensor(std::string name, const std::vector<Index>& indices):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate_zeroed(m_size);
}

inline Tensor::Tensor(std::string name, const std::vector<Index>& indices, const double initial_value):
    m_name{std::move(name)}
{
    std::vector<VarianceQualifiedIndex> vq_indices;
    for (const auto& index: indices) vq_indices.emplace_back(index, COVARIANT);
    from_variance_qualified_indices(vq_indices);
    m_data = impl::allocate(m_size, initial_value);
}

inline Tensor::Tensor(const double initial_value):
    Tensor{impl::TENSOR_DEFAULT_NAME, initial_value}
{}

inline Tensor::Tensor(std::string name, const double initial_value):
    m_name{std::move(name)}
{
    m_data = impl::allocate(1);
    *m_data = initial_value;
}

inline Tensor::Tensor(const Tensor& other):
    m_dimensions{other.m_dimensions},
    m_size{other.m_size},
    m_name{other.m_name},
    m_tensor_class{other.m_tensor_class}
{
    m_data = impl::allocate(m_size);
    impl::copy(m_data.get(), other.m_data.get(), m_size);
}

inline Tensor& Tensor::operator=(const Tensor& other) {
    if (other.m_data != m_data) { // if this isn't copy-to-self
        m_name = other.m_name;
        m_dimensions = other.m_dimensions;
        m_size = other.m_size;
        m_tensor_class = other.m_tensor_class;

        impl::DoublePtr new_data = impl::allocate(m_size);
        impl::copy(new_data.get(), other.m_data.get(), m_size);
        m_data.swap(new_data);
    }

    return *this;
}

inline Tensor::Tensor(Tensor&& other) noexcept:
    m_dimensions{std::move(other.m_dimensions)},
    m_size{other.m_size},
    m_data{std::move(other.m_data)},
    m_name{std::move(other.m_name)},
    m_tensor_class{other.m_tensor_class}
{
    other.m_data = nullptr;
}

inline Tensor& Tensor::operator=(Tensor&& other) noexcept {
    if (other.m_data != m_data) { // if this isn't move-to-self
        m_name = std::move(other.m_name);
        m_dimensions = std::move(other.m_dimensions);
        m_size = other.m_size;
        m_tensor_class = other.m_tensor_class;

        m_data = std::move(other.m_data);
        other.m_data = nullptr;
    }

    return *this;
}

inline Tensor::operator double() const {
    impl::deny(m_size > 1, "Attempt to convert non-scalar tensor to double");
    return *m_data;
}

inline View Tensor::operator[](impl::Indexables indices) const {
    impl::deny(indices.size() != m_dimensions.size(), "Indexing dimension mismatch");

    size_t offset{0};
    size_t width{1};
    impl::Dimensions passed_indices;
    for (size_t i = 0; i < indices.size(); ++i) {
        if (std::holds_alternative<Index>(indices[i])) {
            impl::deny(std::get<Index>(indices[i]).size() > m_dimensions[i].index.size(),
                            "Index size mismatch");
            passed_indices.emplace_back(
                std::get<Index>(indices[i]),
                m_dimensions[i].variance,
                width
            );
            width *= std::get<Index>(indices[i]).size();
        }
        else {
            impl::deny(std::get<int>(indices[i]) >= m_dimensions[i].index.size(),
                            "Index size mismatch");
            offset += std::get<int>(indices[i]) * width;
        }
    }

    return View{*this, m_data.get() + offset, passed_indices};
}

inline double& Tensor::operator[](const std::vector<int>& indices) const {
    impl::deny(indices.size() > m_dimensions.size(), "Indexing dimension mismatch");
    auto data = m_data.get();

    int n = 0;
    for(const auto index : indices) {
        impl::deny(index < 0, "Indices cannot be less than zero");
        impl::deny(index >= m_dimensions[n].index.size(), "Index size mismatch");

        data += m_dimensions[n].width * index;
        ++n;
    }

    return *data;
}

[[nodiscard]] inline std::string Tensor::name() const {return m_name;}
[[nodiscard]] inline size_t Tensor::size() const {return m_size;}
[[nodiscard]] inline size_t Tensor::size(const int dimension) const {return m_dimensions[dimension].index.size();}
[[nodiscard]] inline int Tensor::rank() const {return static_cast<int> (m_dimensions.size());}
[[nodiscard]] inline bool Tensor::is_scalar() const {return m_size == 1;}
[[nodiscard]] inline double Tensor::get_scalar() const {return *m_data;}
[[nodiscard]] inline bool Tensor::is_metric() const {return m_tensor_class == impl::METRIC_TENSOR;}

[[nodiscard]] inline std::vector<Index> Tensor::indices() const {
    std::vector<Index> indices;
    for (const auto& dimension: m_dimensions) indices.emplace_back(dimension.index);
    return indices;
}

[[nodiscard]] inline std::vector<VarianceQualifiedIndex> Tensor::qualified_indices() const {
    std::vector<VarianceQualifiedIndex> qualified_indices;
    for (const auto& dimension: m_dimensions) qualified_indices.emplace_back(dimension.index, dimension.variance);
    return qualified_indices;
}

[[nodiscard]] inline Variance Tensor::variance(const Index& index) const {
    for (const auto& dimension: m_dimensions) {
        if (dimension.index == index){
            return dimension.variance;
        }
    }
    throw TensorLogicError(
        "Missing index when finding variance!"
    ); // can't use deny() as there is nothing to return
}

[[nodiscard]] inline Variance Tensor::variance(const int index) const {
    impl::deny(static_cast<size_t> (index) >= m_dimensions.size(), "Index out of bounds");
    return m_dimensions[index].variance;
}

[[nodiscard]] inline bool Tensor::has_index(const Index& index) const {
    return std::ranges::any_of(m_dimensions, [&](const auto& dimension) {return dimension.index == index;});
}

[[nodiscard]] inline size_t Tensor::index_position(const Index& index) const {
    for (size_t i=0; i<m_dimensions.size(); ++i) {
        if (m_dimensions[i].index == index){
            return i;
        }
    }
    throw TensorLogicError(
        "Missing index when finding index position"
    ); // can't use deny() as there is nothing to return
}

[[nodiscard]] inline const Index& Tensor::indices(const int index) const {
    impl::deny(static_cast<size_t>(index) >= m_dimensions.size(), "Index out of bounds");
    return m_dimensions[index].index;
}

inline Tensor& Tensor::set_name(const std::string& name) {
    m_name = name;
    return *this;
}

inline Tensor& Tensor::transpose(const Index& first, const Index& second) {
    impl::deny(rank() < 2, "Tensor has too few indices for transposition");
    impl::deny(first == second, "Cannot transpose identical indices");
    impl::deny(first.size() != second.size(), "Cannot transpose indices of different lengths");

    // differentiate between the transposition and static dimensions
    impl::Dimension* dim1{nullptr};
    impl::Dimension* dim2{nullptr};
    impl::Dimensions static_dims;
    for (auto & m_dimension : m_dimensions) {
        if      (m_dimension.index == first)  dim1 = &m_dimension;
        else if (m_dimension.index == second) dim2 = &m_dimension;
        else static_dims.emplace_back(m_dimension);
    }

    impl::deny(!(dim1 && dim2), "Missing index when transposing");

    const auto transpose_size = dim1->index.size(); // NOLINT - we check for nullptr in the line above
    auto transpose_slice = [&transpose_size, &dim1, &dim2](double* data) {
        for (int j = 0; j < transpose_size - 1; ++j) {
            for (int i = j + 1; i < transpose_size; ++i) {
                std::swap(
                    *(data + i*dim1->width + j*dim2->width),
                    *(data + i*dim2->width + j*dim1->width)
                );
            }
        }
    };

    if (static_dims.empty()) transpose_slice(m_data.get());
    else {
        auto static_iter = begin();
        std::vector<int> static_positions(static_dims.size());
        do transpose_slice(static_iter.data());
        while (impl::increment_positions(static_positions, static_dims, static_iter));
    }

    std::swap(dim1->index, dim2->index);
    std::swap(dim1->variance, dim2->variance);
    // the width is intentionally NOT swapped to maintain memory alignment

    return *this;
}

inline Tensor& Tensor::relabel(const Index& old_index, const Index& new_index) {
    for (auto& dimension: m_dimensions) {
        if (dimension.index == old_index) {
            impl::deny(dimension.size() != new_index.size(),
                 "Cannot relabel to an index with different size");
            dimension.index = new_index;
            return *this;
        }
    }
    throw TensorLogicError(
        "Cannot find index to relabel!"
    ); // can't use deny() as there is nothing to return
}

inline Tensor& Tensor::set_variance(const Index& index, const Variance variance) {
    for (auto& dimension: m_dimensions) {
        if (dimension.index == index) {
            dimension.variance = variance;
            return *this;
        }
    }
    throw TensorLogicError(
        "Cannot find index!"
    ); // can't use deny() as there is nothing to return
}

inline Tensor& Tensor::raise(const Index& index) {
    return set_variance(index, CONTRAVARIANT);
}

inline Tensor& Tensor::lower(const Index& index) {
    return set_variance(index, COVARIANT);
}

[[nodiscard]] inline View::iterator Tensor::begin() const {
    return View{*this}.begin();
}

[[nodiscard]] inline View::iterator Tensor::end() const {
    return View{*this}.end();
}

[[nodiscard]] inline View::const_iterator Tensor::cbegin() const {
    return View{*this}.cbegin();
}

[[nodiscard]] inline View::const_iterator Tensor::cend() const {
    return View{*this}.cend();
}

inline impl::ProductOp Tensor::operator-() const {
    return {*this, -1.0, impl::MUL};
}

inline impl::LinkedOp operator+(const Tensor& first, const Tensor& second) {
    return {first, second, impl::ADD};
}

inline impl::LinkedOp operator+(const Tensor& first, const double& second) {
    return {first, second, impl::ADD};
}

inline impl::LinkedOp operator+(const double& first, const Tensor& second) {
    return {first, second, impl::ADD};
}

inline Tensor& operator+=(Tensor& first, const Tensor& second) {
    impl::LinkedOp{first, second, impl::ADD}.populate(first, false);
    return first;
}

inline double& operator+=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot add double and non-scalar tensor");
    return first += *second.m_data;
}

inline Tensor& operator+=(Tensor& first, const double& second) {
    impl::deny(first.m_size != 1, "Cannot add non-scalar tensor and double");
    *first.m_data += second;
    return first;
}

inline impl::LinkedOp operator-(const Tensor& first, const Tensor& second) {
    return {first, second, impl::SUB};
}

inline impl::LinkedOp operator-(const Tensor& first, const double& second) {
    return {first, second, impl::SUB};
}

inline impl::LinkedOp operator-(const double& first, const Tensor& second) {
    return {first, second, impl::SUB};
}

inline Tensor& operator-=(Tensor& first, const Tensor& second) {
    impl::LinkedOp{first, second, impl::SUB}.populate(first, false);
    return first;
}

inline double& operator-=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot subtract double and non-scalar tensor");
    return first -= *second.m_data;
}

inline Tensor& operator-=(Tensor& first, const double& second) {
    impl::deny(first.m_size != 1, "Cannot subtract non-scalar tensor and double");
    *first.m_data -= second;
    return first;
}

inline impl::ProductOp operator*(const Tensor& first, const Tensor& second) {
    return {first, second, impl::MUL};
}

inline impl::ProductOp operator*(const Tensor& first, const double& second) {
    return {first, second, impl::MUL};
}

inline impl::ProductOp operator*(const double& first, const Tensor& second) {
    return {first, second, impl::MUL};
}

inline Tensor& operator*=(Tensor& first, const Tensor& second) {
    impl::ProductOp{first, second, impl::MUL}.populate(first);
    return first;
}

inline double& operator*=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot assign non-scalar tensor to double!");
    return first *= *second.m_data;
}

inline Tensor& operator*=(Tensor& first, const double& second) {
    impl::ProductOp{first, second, impl::MUL}.populate(first);
    return first;
}

inline impl::ProductOp operator/(const Tensor& first, const Tensor& second) {
    return impl::ProductOp{first, second, impl::DIV};
}

inline impl::ProductOp operator/(const Tensor& first, const double& second) {
    return {first, second, impl::DIV};
}

inline impl::ProductOp operator/(const double& first, const Tensor& second) {
    return {first, second, impl::DIV};
}

inline Tensor& operator/=(Tensor& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot divide by non-scalar tensor!");
    impl::ProductOp{first, *second.m_data, impl::DIV}.populate(first);
    return first;
}

inline double& operator/=(double& first, const Tensor& second) {
    impl::deny(second.m_size != 1, "Cannot divide by non-scalar tensor!");
    return first /= *second.m_data;
}

inline Tensor& operator/=(Tensor& first, const double& second) {
    impl::ProductOp{first, second, impl::DIV}.populate(first);
    return first;
}

inline bool operator==(const Tensor& first, const Tensor& second) {
    try {
        for (const auto& value: Tensor{first - second}) {
            if (value != 0) return false; // indices MUST be linked when comparing
        }
    }
    catch (std::logic_error&) {
        return false;
    }

    return true;
}

inline bool operator==(const Tensor& first, const double& second) {
    if (first.size() != 1) return false;
    return *first.m_data == second;
}

inline bool operator==(const double& first, const Tensor& second) {
    if (second.size() != 1) return false;
    return first == *second.m_data;
}

inline void Tensor::set_tensor_class(const impl::TensorClass tensor_class) {
    m_tensor_class = tensor_class;
}

inline void Tensor::populate_scalar(const double scalar, const bool allocate) {
    if (allocate) { // for LinkedOp assignment, the original tensor is already set up correctly
        m_data = impl::allocate(1);
        m_size = 1;
    }

    *m_data = scalar;
}

// =================================================================================================
//                                                                                 pretty printing |
// =================================================================================================

namespace impl {

inline size_t print_data_width = 5;
inline int print_data_precision = 1;

// every entry in this string must be EXACTLY 5 lines; the name and width of each
// letter must be recorded in the subsequent letter_widths map
const auto letters = R"(
  __ ,
/ _  |
\__, |
\___/

 ___
/ __)
> _)
\___)
 / ,__|
 \ \
 ,  \
| () |
 \__/
,_______
\  ,---.|
 | |   L|
 | |
 |_|





,-,
|/







 ____



 ----

   _
 _| |_
|_   _|
  |_|

\|/
/|\



_.
\ \
 \ \
  \ \
   \_\
    ,_,
   / /
  / /
 / /
/_/

 _____
|_____|
 _____
|_____|
(_) ,_,
   / /
  / /
 / / _
/_/ ( )

    _
  /  \
 / /\ \
/_/  \_\



 _
'_'



,_,
|/
  /
 |
 |
 |
  \
\
 |
 |
 |
/
 ,-
 |
 |
 |
 '_
-.
 |
 |
 |
_'
 ,-
 |
<
 |
 '_
-.
 |
  >
 |
_'
    _
   / \
  / _ \
 / ___ \
/_/   \_\
 ____
|  _ )
|  _ \
| |_) |
|____/
  ____
 / ___|
| |
| |___
 \____|
 ____
|  _ \
| | | |
| |_| |
|____/
 _____
| ____|
|  _|
| |___
|_____|
 _____
|  ___|
| |_
|  _|
|_|
  ____
 / ___|
| |  _
| |_| |
 \____|
 _   _
| |_| |
|  _  |
| | | |
|_| |_|
 ___
|_ _|
 | |
 | |
|___|
   ___
  |__ |
 _  | |
| |_| |
 \___/
 _  __
| |/ /
| ' /
| . \
|_|\_\
 _
| |
| |
| |___
|_____|
 __  __
|  \/  |
| |\/| |
| |  | |
|_|  |_|
 _   _
| \ | |
|  \| |
| |\  |
|_| \_|
  ___
 / _ \
| | | |
| |_| |
 \___/
 ____
|  _ \
| |_) |
|  __/
|_|
  ___
 / _ \
| | | |
| |_| |
 \__\_\
 ____
|  _ \
| |_) |
|  _ <
|_| \_\
 ____
/ ___|
\___ \
 ___) |
|____/
 _____
|_   _|
  | |
  | |
  |_|
 _   _
| | | |
| | | |
| |_| |
 \___/
 _     _
| |   | |
 \ \ / /
  \ V /
   \_/
 _      _
| |    | |
| \ /\ / |
 \ V  V /
  \_/\_/
__  __
\ \/ /
 \  /
 /  \
/_/\_\
__   __
\ \ / /
 \ V /
  | |
  |_|
_____
|__  /
  / /
 / /_
/____|


 __,
/  |
\__|,
 |
 |
 |__
 |  \
,|__/


 ,_
/
\__,
   |
   |
 __|
/  |
\__|,


 __
/__|
\__
  _
 / `
-|-
 |
 |


,__,
\__|
___/
|
|
| _
|/ |
|  |

 .
 _
 |
 \_
  .
  _
  |
  |
\_/
|
|__
|  \
|__/
|  \
 |
 |
 |
 |
 \___


, _  _
|/ |/ |
|  |  |


, _
|/ |
|  |


 __
/  \
\__/


,__,
|__/
|


,__,
\__|
   |/


. ,-
 |
 |


 __
|__,
.__|

_|_
 |
 |
 \_


_   _
|   |
|__/|,


.    ,
 \  /
  \/


.        ,
 \  /\  /
  \/  \/


\ /
 x
/ \


,   _
\__/|
 ___/


___
 \
__\
)";

// controls the width of each letter when printed; this is necessary to
// make every line of each letter the same width to prevent misalignment
const std::vector<std::pair<int, int>> letter_widths = {
    {METRIC_TENSOR, 6},
    {LEVI_CIVITA_SYMBOL, 5},
    {KRONECKER_DELTA, 7},
    {CHRISTOFFEL_SYMBOL, 9},
    {' ', 3},
    {'\'', 3},
    {'_', 6},
    {'-', 6},
    {'+', 7},
    {'*', 3},
    {'\\', 6},
    {'/', 6},
    {'=', 7},
    {'%', 7},
    {'^', 8},
    {'.', 3},
    {',', 3},
    {'(', 3},
    {')', 3},
    {'[', 3},
    {']', 3},
    {'{', 3},
    {'}', 3},
    {'A', 9},
    {'B', 7},
    {'C', 7},
    {'D', 7},
    {'E', 7},
    {'F', 7},
    {'G', 7},
    {'H', 7},
    {'I', 5},
    {'J', 7},
    {'K', 6},
    {'L', 7},
    {'M', 8},
    {'N', 7},
    {'O', 7},
    {'P', 7},
    {'Q', 7},
    {'R', 7},
    {'S', 7},
    {'T', 7},
    {'U', 7},
    {'V', 9},
    {'W', 10},
    {'X', 6},
    {'Y', 7},
    {'Z', 6},
    {'a', 5},
    {'b', 5},
    {'c', 4},
    {'d', 5},
    {'e', 4},
    {'f', 4},
    {'g', 4},
    {'h', 4},
    {'i', 4},
    {'j', 4},
    {'k', 4},
    {'l', 5},
    {'m', 7},
    {'n', 4},
    {'o', 4},
    {'p', 4},
    {'q', 5},
    {'r', 4},
    {'s', 4},
    {'t', 4},
    {'u', 7},
    {'v', 6},
    {'w', 10},
    {'x', 3},
    {'y', 5},
    {'z', 4}
};

inline void add_letter(std::vector<std::string>& lines, const int letter) {
    auto* letter_ptr = letters;

    for (auto [cur_letter, width]: letter_widths) {
        if (cur_letter == letter) {
            // we found the letter we need, so output it and return
            for (auto& line: lines) {
                for (int i = 0; i < width; ++i) {
                    if (*letter_ptr != '\n') {
                        line += *letter_ptr;
                        ++letter_ptr;
                    }
                    else line += ' ';
                }
                ++letter_ptr;
            }
            return;
        }

        // step 5 lines to get to the next letter
        int step = 5;
        while (step) {
            if (*letter_ptr == '\n') --step;
            ++letter_ptr;
        }
    }
    // if we didn't find the letter, just skip
}

inline void add_header_name(
    const std::string& name,
    std::vector<std::string>& head_lines,
    const TensorClass tensor_class
) {
    if (tensor_class != TENSOR) { // some tensors have special names, e.g. the kronecker delta
        add_letter(head_lines, tensor_class);
    }
    else {
        for (const auto letter: name) {
            add_letter(head_lines, letter);
        }
    }
}

inline void add_header_indices(const Dimensions& dimensions, std::vector<std::string>& header) {
    for (auto& dimension: dimensions) {
        std::string blank;
        for(size_t i=0; i<dimension.index.name().size(); ++i) blank += " ";

        if (dimension.variance == CONTRAVARIANT){ // Upper index
            header[1] += dimension.index.name() + " ";
            header[3] += blank + " ";
            header[4] += blank + " ";
        }
        else { // Lower index
            header[1] += blank + " ";
            header[3] += blank + " ";
            header[4] += dimension.index.name() + " ";
        }
    }
}

inline void write_header(
    std::ostream& stream,
    const Dimensions& dimensions,
    const std::string& name,
    const TensorClass tensor_type
) {
    /** Outputs the "title" of the tensor in fancy lettering with the indices in their correct positions
     * e.g.   ______
     *       |__  __|
     *         | |    mu
     *         | |          =
     *         |_|      nu
     */

    auto add_space = [](std::vector<std::string>& header) {
        for(auto& line: header) line += " ";
    };

    auto add_equals_sign = [&](std::vector<std::string>& header) {
        if (tensor_type == METRIC_TENSOR) header[3] += "  ";
        else header[3] += " =";
    };

    auto output_header = [&](const std::vector<std::string>& header) {
        for (int i = 0; i < 5; ++i) stream << header[i] << "\n";
    };

    std::vector<std::string> header{5};

    add_header_name(name, header, tensor_type);
    add_space(header);
    add_header_indices(dimensions, header);
    add_equals_sign(header);
    output_header(header);
}

inline void write_subtitle(std::ostream& stream, const Tensor& tensor, const TensorClass print_type) {
    /** writes a small note of what "sort" of tensor we have
     * e.g. "Rank 5 Tensor", "Column Vector", etc.
     */
    if (print_type == METRIC_TENSOR) stream << "Metric Tensor\n\n";
    else if (print_type == LEVI_CIVITA_SYMBOL) stream << "Levi-Civita Symbol\n\n";
    else if (print_type == KRONECKER_DELTA) stream << "Kronecker Delta\n\n";
    else {
        if (const auto rank = tensor.rank(); rank == 0) stream << "Scalar Tensor\n\n";
        else if (rank == 1) {
            if (tensor.variance(0) == CONTRAVARIANT) stream << "Column";
            else stream << "Row";
            stream << " Vector\n\n";
        }
        else if (rank == 2) stream << " Matrix Tensor\n\n";
        else stream << "Rank " << rank << " Tensor\n\n";
    }
}

inline std::string format_value(const double value) {
    std::stringstream stream;
    stream << std::fixed << std::setprecision(print_data_precision) << value;
    auto result = stream.str();

    // pad any strings shorter than print_data_width
    if (result.size() < print_data_width) {
        result.insert(0, std::string(print_data_width - result.size(), ' '));
    }

    // truncate any strings longer than print_data_width and add the truncation character (~)
    else if (result.size() > print_data_width) {
        result = result.substr(0, print_data_width -1) + "~"; // -1 to accommodate ~
    }

    return result;
}

inline void output_row(
    std::ostream& stream,
    const double* data,
    const size_t size
) {
    // for scalar and row vector
    stream << "< ";
    for (size_t i=0; i<size; ++i) stream << format_value(data[i]) << " ";
    stream << ">" << "\n";
}

inline void output_column(
    std::ostream& stream,
    const double* data,
    const size_t size
) {
    // for scalar and row vector
    stream << " /" << format_value(data[0]) << "\\\n" ;
    for (size_t i=1; i<size-1; ++i) stream << "| " << format_value(data[i]) << " |\n";
    stream << " \\" << format_value(data[size-1]) << "/\n" ;
}

inline void output_grid(
    const std::vector<std::vector<std::string>>& value_matrix,
    std::ostream& stream,
    const int total_width,
    const int total_height,
    const int i_size,
    const int k_size
) {
    std::vector<std::stringstream> output_rows(total_height + 2); // +2 for the bracket heads at the top and bottom
    const int n_whitespace = i_size * k_size + k_size; // spaces between characters + spaces between slices
    const int total_fields_length = total_width * static_cast<int>(print_data_width);
    const int total_matrix_width = total_fields_length + n_whitespace - 1;

    // add the starting bracket
    output_rows[0] << " /";
    for (int ii=1; ii<total_height+1; ++ii) output_rows[ii] << "|";
    output_rows[total_height+1] << " \\";

    // add the value data in between
    output_rows[0] << std::string(total_matrix_width, ' ');
    for (size_t ii=1; ii<output_rows.size()-1; ++ii) {
        for (int jj=0; jj<total_width; ++jj) {
            if (jj % i_size == 0) output_rows[ii] << " "; // adds an extra space before every slice
            output_rows[ii] << value_matrix[ii - 1][jj] << " "; // note the space between every field
        }
    }
    output_rows[total_height+1] << std::string(total_matrix_width, ' ');

    // add the ending bracket
    output_rows[0] << "\\\n";
    for (int ii=1; ii<total_height+1; ++ii) output_rows[ii] << " |\n";
    output_rows[total_height+1] << "/\n";

    // finally, output all the rows
    for (auto& row: output_rows) stream << row.str();
}

inline void output_234(const Tensor& tensor, const Dimensions& dimensions, std::ostream& stream) {
    // for tensors of rank 2, 3, or 4 (by far the most complicated case)

    const int i_size = dimensions[0].index.size();
    const int j_size = dimensions[1].index.size();
    const int k_size = dimensions.size() >= 3 ? dimensions[2].index.size() : 1;

    const int n_major_row = dimensions.size() == 4 ? dimensions[3].size() : 1; // number of big grids to output
    const int n_squares = dimensions.size() >= 3 ? dimensions[2].size() : 1; // number of squares in each grid

    // create a set of indices to slice each square out of the tensor
    Indexables indices;
    switch (dimensions.size()) {
        default:
            indices.insert(indices.begin(), -1);
            [[fallthrough]];
        case 3:
            indices.insert(indices.begin(), -1);
            [[fallthrough]];
        case 2:
            indices.insert(indices.begin(), dimensions[1].index);
            indices.insert(indices.begin(), dimensions[0].index);
            break;
    }

    // make a string matrix to store the values in
    // storing in height-by-width form makes the output stage easier
    const int total_width = i_size * n_squares;
    const int total_height = j_size + n_squares - 1;
    std::vector value_matrix(
        total_height,
        std::vector(total_width, std::string(print_data_width, ' '))
    );

    for (int i=0; i<n_major_row; ++i) {
        // for each major row
        if (dimensions.size() == 4) indices[3] = i;

        int start_height = total_height - j_size;
        int start_width = 0;
        for (int j=0; j<n_squares; ++j) { // for each square in the major row
            if (dimensions.size() >= 3) indices[2] = j;

            // slice the tensor and fill in the slice's values starting at (start_height, start_width)
            Tensor square = tensor[indices];
            for (int jj=0; jj<j_size; ++jj) {
                for (int ii=0; ii<i_size; ++ii) {
                    value_matrix[start_height+jj][start_width + ii] = format_value(square[ii, jj]);
                }
            }

            // move the starting position to the next square
            start_width += i_size;
            --start_height;
        }

        output_grid(value_matrix, stream, total_width, total_height, i_size, k_size);
    }
}

} // namespace impl

/**
 * @brief Set the width of the fields when geometric pretty-printing
 *
 * Any values on the tensor longer than this width will be truncated with a ~ symbol.
 */
inline void set_print_data_width(const int width) {
    // sets the width of the data elements when pretty printing
    impl::print_data_width = width;
}

/**
 * @brief Set the number of significant figures to round values to when pretty-printing
 */
inline void set_print_precision(const int precision) {
    // set the precision of the output stream when pretty printing
    impl::print_data_precision = precision;
}

inline void write_data(std::ostream& stream, const Tensor& tensor) {
    // outputs just the tensor data in pretty format
    if(tensor.m_dimensions.empty()) impl::output_row(stream, tensor.m_data.get(), tensor.m_size);
    else if(tensor.m_dimensions.size() == 1) {
        if (tensor.variance(0) == CONTRAVARIANT) impl::output_column(stream, tensor.m_data.get(), tensor.m_size);
        else impl::output_row(stream, tensor.m_data.get(), tensor.m_size);
    }
    else if (tensor.m_dimensions.size() <= 4) impl::output_234(tensor, tensor.m_dimensions, stream);
    else tensor.dump(stream);
}

inline std::ostream& pretty_print(std::ostream& ostream, const Tensor& tensor) {
    // display the tensor in detailed geometric format, including name and index information
    impl::write_header(ostream, tensor.m_dimensions, tensor.name(), tensor.m_tensor_class);
    impl::write_subtitle(ostream, tensor, tensor.m_tensor_class);
    write_data(ostream, tensor);
    return ostream;
}

}  // namespace varitensor

inline std::ostream& operator<<(std::ostream& stream, const varitensor::Tensor& tensor) {
    return varitensor::pretty_print(stream, tensor);
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::LinkedOp operator+(const T& first, const U& second) {
    return {first, second, varitensor::impl::ADD};
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::LinkedOp operator-(const T& first, const U& second) {
    return {first, second, varitensor::impl::SUB};
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::ProductOp operator*(const T& first, const U& second) {
    return {first, second, varitensor::impl::MUL};
}

template<varitensor::impl::ExpressionOperand_c T, varitensor::impl::ExpressionOperand_c U>
varitensor::impl::ProductOp operator/(const T& first, const U& second) {
    return {first, second, varitensor::impl::DIV};
}

#endif // VARITENSOR_H
